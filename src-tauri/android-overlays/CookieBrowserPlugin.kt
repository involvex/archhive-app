package com.archhive.app

import android.annotation.SuppressLint
import android.app.Activity
import android.app.Dialog
import android.graphics.Bitmap
import android.graphics.Color
import android.os.Handler
import android.os.Looper
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
import org.json.JSONObject
import org.json.JSONTokener

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

    class ResolveArgs @JvmOverloads constructor(
        var url: String = "",
        var title: String = "Opening video page",
    )

    class FollowArgs @JvmOverloads constructor(
        var url: String = "",
        var referer: String = "",
    )

    /**
     * Page resolver used when a site shows a Cloudflare / bot check to the
     * embedded engines (yt-dlp, the Rust HTTP client). The page is opened in
     * a real WebView — the check passes there (or the user taps it once) —
     * and Rust gets the rendered HTML, cookies and User-Agent back. The
     * dialog stays open so [followRedirect] can resolve session-bound links
     * (KVS `get_file`) inside the same browser session; [closeResolver]
     * dismisses it. One resolver at a time.
     */
    private inner class Resolver(
        val dlg: Dialog,
        val webView: WebView,
        val status: TextView,
    ) {
        var pending: Invoke? = null
        var mode = "page" // page | idle | follow
        var followTarget: String? = null
        var timeout: Runnable? = null
        var poll: Runnable? = null
    }

    @Volatile private var resolver: Resolver? = null
    private val main = Handler(Looper.getMainLooper())

    private fun isChallengeTitle(t: String): Boolean =
        Regex("just a moment|attention required|checking your browser|ddos-guard|security verification|verify you are human", RegexOption.IGNORE_CASE)
            .containsMatchIn(t)

    private fun settle(r: Resolver, ok: JSObject?, error: String?) {
        val inv = r.pending ?: return
        r.pending = null
        r.timeout?.let { main.removeCallbacks(it) }
        r.timeout = null
        if (ok != null) inv.resolve(ok) else inv.reject(error ?: "failed")
    }

    private fun closeResolverNow(error: String?) {
        val r = resolver ?: return
        resolver = null
        r.poll?.let { main.removeCallbacks(it) }
        settle(r, null, error ?: "closed")
        try {
            r.webView.stopLoading()
            r.webView.destroy()
        } catch (_: Exception) {
        }
        try {
            r.dlg.dismiss()
        } catch (_: Exception) {
        }
    }

    private fun checkPage(r: Resolver) {
        if (resolver !== r || r.mode != "page") return
        val probe = "(function(){var t=document.title||'';" +
            "var cf=!!document.querySelector('#challenge-form,#challenge-running,#cf-challenge-running,.cf-turnstile,[name=cf-turnstile-response],#turnstile-wrapper');" +
            "return JSON.stringify({t:t,cf:cf,ready:document.readyState});})()"
        r.webView.evaluateJavascript(probe) { raw ->
            if (resolver !== r || r.mode != "page") return@evaluateJavascript
            val info = try {
                JSONObject(JSONTokener(raw).nextValue() as String)
            } catch (_: Exception) {
                null
            } ?: return@evaluateJavascript
            val title = info.optString("t")
            if (info.optBoolean("cf") || isChallengeTitle(title)) {
                r.status.text = "Security check — tap the box if one appears. This closes by itself."
                return@evaluateJavascript
            }
            if (info.optString("ready") != "complete") return@evaluateJavascript
            r.mode = "idle"
            r.status.text = "Loading video…"
            // Give the site's player scripts a moment to fill in the source.
            main.postDelayed({
                if (resolver !== r) return@postDelayed
                r.webView.evaluateJavascript("document.documentElement.outerHTML") { htmlRaw ->
                    if (resolver !== r) return@evaluateJavascript
                    val html = try {
                        JSONTokener(htmlRaw).nextValue() as? String ?: ""
                    } catch (_: Exception) {
                        ""
                    }
                    val pageUrl = r.webView.url ?: ""
                    val ret = JSObject()
                    ret.put("html", html)
                    ret.put("url", pageUrl)
                    ret.put("userAgent", r.webView.settings.userAgentString)
                    ret.put("cookies", CookieManager.getInstance().getCookie(pageUrl) ?: "")
                    settle(r, ret, null)
                }
            }, 1500)
        }
    }

    private fun finishFollow(r: Resolver, url: String) {
        if (r.mode != "follow") return
        r.mode = "idle"
        try {
            r.webView.stopLoading()
        } catch (_: Exception) {
        }
        val ret = JSObject()
        ret.put("url", url)
        ret.put("cookies", CookieManager.getInstance().getCookie(url) ?: "")
        settle(r, ret, null)
    }

    @SuppressLint("SetJavaScriptEnabled")
    @Command
    fun resolvePage(invoke: Invoke) {
        val args = invoke.parseArgs(ResolveArgs::class.java)
        if (!args.url.startsWith("https://") && !args.url.startsWith("http://")) {
            invoke.reject("Invalid page URL")
            return
        }
        activity.runOnUiThread {
            try {
                closeResolverNow("superseded")
                val cookieManager = CookieManager.getInstance()
                cookieManager.setAcceptCookie(true)
                val dlg = Dialog(activity, android.R.style.Theme_Black_NoTitleBar_Fullscreen)
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
                val status = TextView(activity).apply {
                    text = "${args.title}\nOpening the page… this closes by itself."
                    setTextColor(Color.WHITE)
                    textSize = 14f
                    maxLines = 3
                    setPadding(16, 0, 16, 0)
                }
                bar.addView(cancel)
                bar.addView(status, LinearLayout.LayoutParams(0, ViewGroup.LayoutParams.WRAP_CONTENT, 1f))
                val webView = WebView(activity)
                val r = Resolver(dlg, webView, status)
                webView.apply {
                    settings.javaScriptEnabled = true
                    settings.domStorageEnabled = true
                    settings.databaseEnabled = true
                    settings.loadWithOverviewMode = true
                    settings.useWideViewPort = true
                    settings.mediaPlaybackRequiresUserGesture = true
                    settings.setSupportMultipleWindows(false)
                    settings.userAgentString = settings.userAgentString.replace("; wv", "")
                    webViewClient = object : WebViewClient() {
                        override fun shouldOverrideUrlLoading(view: WebView, request: WebResourceRequest): Boolean {
                            val scheme = request.url.scheme ?: return true
                            if (scheme != "http" && scheme != "https") return true
                            if (r.mode == "follow" && request.isForMainFrame && request.isRedirect) {
                                finishFollow(r, request.url.toString())
                                return true
                            }
                            return false
                        }

                        override fun onPageStarted(view: WebView, url: String?, favicon: Bitmap?) {
                            // A followed link that did not redirect (or whose
                            // redirect skipped shouldOverrideUrlLoading).
                            if (r.mode == "follow" && url != null) finishFollow(r, url)
                        }

                        override fun onPageFinished(view: WebView, url: String?) {
                            if (r.mode == "page") main.postDelayed({ checkPage(r) }, 400)
                        }
                    }
                }
                cookieManager.setAcceptThirdPartyCookies(webView, true)
                root.addView(bar, LinearLayout.LayoutParams(ViewGroup.LayoutParams.MATCH_PARENT, ViewGroup.LayoutParams.WRAP_CONTENT))
                root.addView(webView, LinearLayout.LayoutParams(ViewGroup.LayoutParams.MATCH_PARENT, 0, 1f))
                dlg.setContentView(root)
                cancel.setOnClickListener { closeResolverNow("cancelled") }
                dlg.setOnCancelListener { closeResolverNow("cancelled") }

                r.pending = invoke
                resolver = r
                // Challenge pages often finish via JS without a new page load.
                val poll = object : Runnable {
                    override fun run() {
                        if (resolver !== r) return
                        if (r.mode == "page") checkPage(r)
                        main.postDelayed(this, 2000)
                    }
                }
                r.poll = poll
                main.postDelayed(poll, 2500)
                val timeout = Runnable { closeResolverNow("timed out waiting for the page") }
                r.timeout = timeout
                main.postDelayed(timeout, 120_000)
                dlg.show()
                webView.loadUrl(args.url)
            } catch (e: Exception) {
                Log.e(TAG, "failed to open page resolver", e)
                invoke.reject("Could not open the in-app browser: ${e.message}")
            }
        }
    }

    /** Load [FollowArgs.url] in the resolver's session and report where it redirects. */
    @Command
    fun followRedirect(invoke: Invoke) {
        val args = invoke.parseArgs(FollowArgs::class.java)
        activity.runOnUiThread {
            val r = resolver
            if (r == null) {
                invoke.reject("resolver closed")
                return@runOnUiThread
            }
            settle(r, null, "superseded")
            r.pending = invoke
            r.mode = "follow"
            r.followTarget = args.url
            r.status.text = "Loading video…"
            val timeout = Runnable { finishFollow(r, args.url) }
            r.timeout = timeout
            main.postDelayed(timeout, 20_000)
            val headers = if (args.referer.isNotBlank()) mapOf("Referer" to args.referer) else emptyMap()
            r.webView.loadUrl(args.url, headers)
        }
    }

    @Command
    fun closeResolver(invoke: Invoke) {
        activity.runOnUiThread {
            closeResolverNow("closed")
            invoke.resolve()
        }
    }

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
