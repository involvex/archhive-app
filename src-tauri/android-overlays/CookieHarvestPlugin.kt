package com.archhive.app

import android.app.Activity
import android.content.Intent
import android.graphics.Bitmap
import android.graphics.Color
import android.net.Uri
import android.os.Bundle
import android.os.Handler
import android.os.Looper
import android.webkit.CookieManager
import android.webkit.WebResourceError
import android.webkit.WebResourceRequest
import android.webkit.WebView
import android.webkit.WebViewClient
import android.widget.Button
import android.widget.LinearLayout
import android.widget.TextView
import android.widget.Toast
import app.tauri.annotation.Command
import app.tauri.annotation.TauriPlugin
import app.tauri.plugin.Invoke
import app.tauri.plugin.JSObject
import app.tauri.plugin.Plugin

/**
 * Cookie extractor flow, native half.
 *
 * ArcHive opens the site's login page in [CookieLoginActivity] (an ordinary Android
 * WebView inside the app sandbox — no root, no external browser permissions). While
 * the user logs in, the activity polls the global [CookieManager] (single process,
 * shared with the Tauri webview). The frontend polls `takeHarvest` (take + clear,
 * same pull model as ShareIntentPlugin) and writes the result into the encrypted
 * cookie vault via `save_site_cookies`.
 *
 * Harvesting is **manual on purpose**. An earlier revision auto-submitted as soon as
 * the cookie string changed, which misfired on multi-step logins: e.g. Stripchat via
 * the X/Twitter OAuth flow, where the X cookies land *before* the user finishes the
 * Stripchat step, or Cloudflare/consent/analytics cookies landing right after load.
 * The window now stays open until the user taps **Save cookies** (or cancels), so a
 * half-finished login can never overwrite a good session. Cancel/Back/timeout save
 * nothing.
 *
 * Platform note: `android.webkit.CookieManager.getCookie()` only returns non-HttpOnly
 * cookies. Sites that keep ALL auth state in HttpOnly cookies cannot be captured in a
 * WebView — documented limitation; the desktop Cookie-Editor / DevTools import paths
 * (which see HttpOnly) cover those.
 */

/** Terminal harvest states (string values cross the Kotlin → Rust boundary). */
object HarvestState {
    /** User tapped "Save cookies" — import these. */
    const val HARVEST = "harvest"

    /** User tapped Cancel or Back, or the page never let them log in. Save nothing. */
    const val CANCELLED = "cancelled"

    /** 10 minute limit hit. Save nothing. */
    const val TIMEOUT = "timeout"
}

/** Result handed between the harvest Activity and the plugin (take + clear). */
data class Harvest(
    /** One of [HarvestState]. */
    val state: String,
    val siteId: String,
    /** "k=v; k2=v2" cookie header for the site session domain. Empty for cancelled/timeout. */
    val cookieHeader: String,
    /** Host of the login page, used as the Netscape cookie domain. */
    val domain: String,
)

@TauriPlugin
class CookieHarvestPlugin(private val activity: Activity) : Plugin(activity) {

    companion object {
        @Volatile
        private var pending: Harvest? = null

        /** Called by [CookieLoginActivity] when the window closes. */
        @JvmStatic
        @Synchronized
        fun store(harvest: Harvest) {
            pending = harvest
        }

        /** Take + clear. Called from the `takeHarvest` command. */
        @Synchronized
        fun take(): Harvest? {
            val current = pending
            pending = null
            return current
        }
    }

    /**
     * Command payload. Plain class with a JVM no-arg constructor + setter so
     * Jackson (no Kotlin module on the Tauri side) can bind {"siteId","url"}.
     */
    class CookieLoginArgs @JvmOverloads constructor(
        var siteId: String = "",
        var url: String = "",
    )

    /**
     * Open the site login page in an in-app WebView. Payload: { siteId, url }.
     */
    @Command
    fun openCookieLogin(invoke: Invoke) {
        val payload = invoke.parseArgs(CookieLoginArgs::class.java)
        val siteId = payload.siteId
        val url = payload.url
        if (siteId.isBlank() || url.isBlank()) {
            invoke.reject("Cookie harvest requires siteId and url")
            return
        }
        try {
            val intent = Intent(activity, CookieLoginActivity::class.java)
            // Started from the Tauri activity context; harmless for an Activity but
            // required if a non-Activity context ever calls this.
            intent.addFlags(Intent.FLAG_ACTIVITY_NEW_TASK)
            intent.putExtra(CookieLoginActivity.EXTRA_SITE_ID, siteId)
            intent.putExtra(CookieLoginActivity.EXTRA_URL, url)
            activity.startActivity(intent)
            invoke.resolve()
        } catch (e: Exception) {
            invoke.reject("Could not open cookie login window: ${e.message}")
        }
    }

    /**
     * Take + clear the harvest state.
     *
     * Returns `{ state, siteId, cookieHeader, domain }` where `state` is `"pending"`
     * while the window is still open, otherwise one of [HarvestState]. The explicit
     * terminal states let the frontend stop polling the moment the user cancels or
     * the window times out (instead of waiting out its own timeout).
     */
    @Command
    fun takeHarvest(invoke: Invoke) {
        val ret = JSObject()
        val harvest = take()
        if (harvest == null) {
            ret.put("state", "pending")
            ret.put("siteId", "")
            ret.put("cookieHeader", "")
            ret.put("domain", "")
        } else {
            ret.put("state", harvest.state)
            ret.put("siteId", harvest.siteId)
            ret.put("cookieHeader", harvest.cookieHeader)
            ret.put("domain", harvest.domain)
        }
        invoke.resolve(ret)
    }
}

/**
 * In-app login window that watches the session cookies of one site.
 *
 * Deliberately manual: the user taps **Save cookies** once they are actually logged
 * in. The poll loop only drives the status line (including a "cookies changed" hint),
 * it never closes the window by itself — auto-detection kept firing early on
 * multi-step / OAuth logins and on post-load Cloudflare and analytics cookies.
 *
 * The status line reacts to our own cookie reads only, using a **fixed origin URL**
 * ([originUrl]): reading `getCookie(webView.url)` made the value change on its own as
 * the page redirected between paths.
 */
class CookieLoginActivity : Activity() {

    companion object {
        const val EXTRA_SITE_ID = "arc_hive_site_id"
        const val EXTRA_URL = "arc_hive_url"
        private const val POLL_INTERVAL_MS = 1_000L
        /** Unchanged cookie polls required before we call the page "settled". */
        private const val SETTLE_POLLS = 6
        private const val TIMEOUT_MS = 10 * 60 * 1000L
    }

    private val mainHandler = Handler(Looper.getMainLooper())
    private lateinit var webView: WebView
    private lateinit var statusView: TextView
    private var siteId: String = ""
    private var startUrl: String = ""
    /** scheme://host — a stable cookie query target for the whole session. */
    private var originUrl: String = ""
    private var baselineCookie: String? = null
    private var settledPolls = 0
    private var announcedChange = false
    private var finished = false

    private fun cookieNow(): String = try {
        CookieManager.getInstance().getCookie(originUrl) ?: ""
    } catch (_: Exception) {
        ""
    }

    private val poll = object : Runnable {
        override fun run() {
            if (finished) return
            val current = cookieNow()
            val base = baselineCookie
            when {
                base == null -> {
                    baselineCookie = current
                    settledPolls = 0
                }
                // Steady: count toward the settle window.
                current == base -> {
                    settledPolls++
                    announcedChange = false
                    if (settledPolls == SETTLE_POLLS) {
                        statusView.text =
                            "Log in, then tap “Save cookies”. (Auto-save is off so a " +
                                "half-finished login can't overwrite a good session.)"
                    }
                }
                // Changed, but the page is still settling (Cloudflare/consent/analytics).
                settledPolls < SETTLE_POLLS -> {
                    baselineCookie = current
                    settledPolls = 0
                    announcedChange = false
                }
                // Settled and changed: almost certainly a login. Ask the user to confirm.
                else -> {
                    baselineCookie = current
                    if (!announcedChange) {
                        announcedChange = true
                        statusView.text = "Cookies changed — tap “Save cookies” to keep this session."
                    }
                }
            }
            mainHandler.postDelayed(this, POLL_INTERVAL_MS)
        }
    }

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        siteId = intent.getStringExtra(EXTRA_SITE_ID) ?: ""
        startUrl = intent.getStringExtra(EXTRA_URL) ?: ""

        if (siteId.isBlank() || startUrl.isBlank()) {
            Toast.makeText(this, "ArcHive: missing site info", Toast.LENGTH_SHORT).show()
            finish()
            return
        }

        originUrl = try {
            val parsed = Uri.parse(startUrl)
            "${parsed.scheme}://${parsed.host}"
        } catch (_: Exception) {
            startUrl
        }

        CookieManager.getInstance().setAcceptCookie(true)

        val root = LinearLayout(this).apply {
            orientation = LinearLayout.VERTICAL
            setBackgroundColor(Color.WHITE)
        }

        // Header: title + live status + explicit Save / Cancel.
        val header = LinearLayout(this).apply {
            orientation = LinearLayout.VERTICAL
            setPadding(28, 24, 28, 20)
            setBackgroundColor(Color.parseColor("#F2F2F2"))
        }
        val title = TextView(this).apply {
            text = "Log in to $siteId"
            textSize = 15f
            setTextColor(Color.parseColor("#111111"))
        }
        statusView = TextView(this).apply {
            text = "Loading the login page…"
            textSize = 12f
            setTextColor(Color.parseColor("#555555"))
        }

        val buttons = LinearLayout(this).apply {
            orientation = LinearLayout.HORIZONTAL
        }
        val save = Button(this).apply {
            text = "Save cookies"
            setOnClickListener { finishWithHarvest(HarvestState.HARVEST) }
        }
        val cancel = Button(this).apply {
            text = "Cancel"
            setOnClickListener { finishWithHarvest(HarvestState.CANCELLED) }
        }
        buttons.addView(
            save,
            LinearLayout.LayoutParams(0, LinearLayout.LayoutParams.WRAP_CONTENT, 1f),
        )
        buttons.addView(
            cancel,
            LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.WRAP_CONTENT,
                LinearLayout.LayoutParams.WRAP_CONTENT,
            ),
        )

        header.addView(title)
        header.addView(statusView)
        header.addView(buttons)
        root.addView(header)

        webView = WebView(this).apply {
            settings.javaScriptEnabled = true
            settings.domStorageEnabled = true
            CookieManager.getInstance().setAcceptThirdPartyCookies(this, true)
            webViewClient = object : WebViewClient() {
                override fun onPageStarted(view: WebView, url: String, favicon: Bitmap?) {
                    super.onPageStarted(view, url, favicon)
                    if (!finished) statusView.text = "Loading…"
                }

                override fun onPageFinished(view: WebView, url: String) {
                    super.onPageFinished(view, url)
                    // Pages that keep refreshing cookies never reach the settle window,
                    // so make the window self-explanatory as soon as a page renders.
                    if (!finished && settledPolls < SETTLE_POLLS && !announcedChange) {
                        statusView.text =
                            "Log in, then tap “Save cookies” when you are done (10 min limit)."
                    }
                }

                override fun onReceivedError(
                    view: WebView,
                    request: WebResourceRequest,
                    error: WebResourceError,
                ) {
                    super.onReceivedError(view, request, error)
                    if (!finished && request.isForMainFrame) {
                        statusView.text =
                            "Page failed to load (${error.description}). Log in anyway or tap Done."
                    }
                }
            }
        }
        root.addView(
            webView,
            LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                0,
                1f,
            ),
        )
        setContentView(root)

        webView.loadUrl(startUrl)
        mainHandler.postDelayed(poll, POLL_INTERVAL_MS)
        mainHandler.postDelayed({ finishWithHarvest(HarvestState.TIMEOUT) }, TIMEOUT_MS)
    }

    @Deprecated("Deprecated in API 33; kept for older devices")
    @Suppress("DEPRECATION")
    override fun onBackPressed() {
        // Back closes without saving — same as Cancel, so a half-finished login can
        // never replace the user's existing session.
        finishWithHarvest(HarvestState.CANCELLED)
    }

    override fun onDestroy() {
        super.onDestroy()
        try {
            CookieManager.getInstance().flush()
        } catch (_: Exception) {
        }
    }

    private fun finishWithHarvest(state: String) {
        if (finished) return
        finished = true
        try {
            mainHandler.removeCallbacks(poll)
        } catch (_: Exception) {
        }
        try {
            CookieManager.getInstance().flush()
        } catch (_: Exception) {
        }
        val header = cookieNow().trim()
        val domain = try {
            Uri.parse(originUrl).host ?: ""
        } catch (_: Exception) {
            ""
        }
        // Only "harvest" carries cookies; cancelled/timeout deliberately save nothing.
        val carried = if (state == HarvestState.HARVEST) header else ""
        CookieHarvestPlugin.store(Harvest(state, siteId, carried, domain))
        Toast.makeText(
            this,
            when {
                state == HarvestState.HARVEST && carried.isNotBlank() ->
                    "ArcHive: session cookies captured"
                state == HarvestState.HARVEST -> "ArcHive: nothing captured"
                state == HarvestState.TIMEOUT -> "ArcHive: cookie login timed out"
                else -> "ArcHive: cancelled"
            },
            Toast.LENGTH_SHORT,
        ).show()
        finish()
    }
}
