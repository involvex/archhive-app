package com.archhive.app

import android.annotation.SuppressLint
import android.app.Activity
import android.app.Dialog
import android.graphics.Bitmap
import android.graphics.Color
import android.util.Log
import android.view.Gravity
import android.view.ViewGroup
import android.webkit.CookieManager
import android.webkit.WebChromeClient
import android.webkit.WebResourceRequest
import android.webkit.WebView
import android.webkit.WebViewClient
import android.widget.Button
import android.widget.LinearLayout
import android.widget.ProgressBar
import android.widget.TextView
import app.tauri.annotation.Command
import app.tauri.annotation.TauriPlugin
import app.tauri.plugin.Invoke
import app.tauri.plugin.JSArray
import app.tauri.plugin.JSObject
import app.tauri.plugin.Plugin

/**
 * In-app login browser used to capture site cookies on Android.
 *
 * Android apps cannot read Chrome's cookie jar, so ArcHive opens its own
 * full-screen WebView on the site's login page. When the user taps "Save
 * cookies", every cookie the WebView holds for the requested URLs, including
 * HttpOnly session cookies, is read from the process-wide [CookieManager]
 * and returned to Rust, which stores them in the encrypted cookie vault.
 *
 * Args:    { url: string, cookieUrls: string[], title: string }
 * Returns: { cookies: [{ url: string, header: string }] }
 * Rejects with "cancelled" when the user backs out.
 */
@TauriPlugin
class CookieBrowserPlugin(private val activity: Activity) : Plugin(activity) {

    companion object {
        private const val TAG = "CookieBrowserPlugin"
    }

    class OpenArgs @JvmOverloads constructor(
        var url: String = "",
        var cookieUrls: Array<String> = emptyArray(),
        var title: String = "Log in",
    )

    @Volatile private var dialog: Dialog? = null

    @SuppressLint("SetJavaScriptEnabled")
    @Command
    fun open(invoke: Invoke) {
        val args = invoke.parseArgs(OpenArgs::class.java)
        if (!args.url.startsWith("https://") && !args.url.startsWith("http://")) {
            invoke.reject("Invalid login URL")
            return
        }
        val cookieUrls = if (args.cookieUrls.isEmpty()) arrayOf(args.url) else args.cookieUrls

        activity.runOnUiThread {
            try {
                dialog?.dismiss()
                val cookieManager = CookieManager.getInstance()
                cookieManager.setAcceptCookie(true)

                val dlg = Dialog(activity, android.R.style.Theme_Black_NoTitleBar_Fullscreen)
                var settled = false

                val root = LinearLayout(activity).apply {
                    orientation = LinearLayout.VERTICAL
                    setBackgroundColor(Color.parseColor("#111114"))
                }
                val bar = LinearLayout(activity).apply {
                    orientation = LinearLayout.HORIZONTAL
                    gravity = Gravity.CENTER_VERTICAL
                    setPadding(16, 12, 16, 12)
                }
                val cancel = Button(activity).apply { text = "Cancel" }
                val title = TextView(activity).apply {
                    text = args.title
                    setTextColor(Color.WHITE)
                    textSize = 15f
                    maxLines = 2
                    setPadding(16, 0, 16, 0)
                }
                val save = Button(activity).apply { text = "Save cookies" }
                bar.addView(cancel)
                bar.addView(title, LinearLayout.LayoutParams(0, ViewGroup.LayoutParams.WRAP_CONTENT, 1f))
                bar.addView(save)

                val progress = ProgressBar(activity, null, android.R.attr.progressBarStyleHorizontal).apply {
                    isIndeterminate = false
                    max = 100
                }

                val webView = WebView(activity).apply {
                    settings.javaScriptEnabled = true
                    settings.domStorageEnabled = true
                    settings.databaseEnabled = true
                    settings.loadWithOverviewMode = true
                    settings.useWideViewPort = true
                    settings.setSupportMultipleWindows(false)
                    // Drop the "; wv" token: some login pages (Google, X)
                    // refuse embedded WebViews that advertise themselves.
                    settings.userAgentString = settings.userAgentString.replace("; wv", "")
                    webViewClient = object : WebViewClient() {
                        override fun shouldOverrideUrlLoading(view: WebView, request: WebResourceRequest): Boolean {
                            val scheme = request.url.scheme ?: return true
                            // Stay inside the WebView for web pages; ignore app deep links.
                            return scheme != "http" && scheme != "https"
                        }

                        override fun onPageStarted(view: WebView, url: String?, favicon: Bitmap?) {
                            title.text = "${args.title}\n${url ?: ""}"
                        }
                    }
                    webChromeClient = object : WebChromeClient() {
                        override fun onProgressChanged(view: WebView, newProgress: Int) {
                            progress.progress = newProgress
                            progress.visibility = if (newProgress >= 100) android.view.View.GONE else android.view.View.VISIBLE
                        }
                    }
                }
                cookieManager.setAcceptThirdPartyCookies(webView, true)

                root.addView(bar, LinearLayout.LayoutParams(ViewGroup.LayoutParams.MATCH_PARENT, ViewGroup.LayoutParams.WRAP_CONTENT))
                root.addView(progress, LinearLayout.LayoutParams(ViewGroup.LayoutParams.MATCH_PARENT, 6))
                root.addView(webView, LinearLayout.LayoutParams(ViewGroup.LayoutParams.MATCH_PARENT, 0, 1f))
                dlg.setContentView(root)

                fun finish(saveCookies: Boolean) {
                    if (settled) return
                    settled = true
                    try {
                        if (saveCookies) {
                            cookieManager.flush()
                            val list = JSArray()
                            for (u in cookieUrls) {
                                val header = cookieManager.getCookie(u)
                                if (!header.isNullOrBlank()) {
                                    val entry = JSObject()
                                    entry.put("url", u)
                                    entry.put("header", header)
                                    list.put(entry)
                                }
                            }
                            val ret = JSObject()
                            ret.put("cookies", list)
                            invoke.resolve(ret)
                        } else {
                            invoke.reject("cancelled")
                        }
                    } catch (e: Exception) {
                        Log.w(TAG, "cookie capture failed", e)
                        invoke.reject("Cookie capture failed: ${e.message}")
                    } finally {
                        webView.stopLoading()
                        webView.destroy()
                        dlg.dismiss()
                        dialog = null
                    }
                }

                save.setOnClickListener { finish(true) }
                cancel.setOnClickListener { finish(false) }
                dlg.setOnCancelListener { finish(false) }
                dlg.setOnKeyListener { _, keyCode, event ->
                    if (keyCode == android.view.KeyEvent.KEYCODE_BACK &&
                        event.action == android.view.KeyEvent.ACTION_UP &&
                        webView.canGoBack()
                    ) {
                        webView.goBack()
                        true
                    } else {
                        false
                    }
                }

                dialog = dlg
                dlg.show()
                webView.loadUrl(args.url)
            } catch (e: Exception) {
                Log.e(TAG, "failed to open login browser", e)
                invoke.reject("Could not open login browser: ${e.message}")
            }
        }
    }
}
