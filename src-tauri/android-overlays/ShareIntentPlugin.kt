package com.archhive.app

import android.app.Activity
import android.content.Intent
import android.util.Log
import app.tauri.annotation.Command
import app.tauri.annotation.TauriPlugin
import app.tauri.plugin.Invoke
import app.tauri.plugin.JSObject
import app.tauri.plugin.Plugin

/**
 * Share-target bridge for Android ACTION_SEND / ACTION_SEND_MULTIPLE (text/plain).
 *
 * ArcHive appears in the Android share sheet. Chrome / Pornhub / any app sharing
 * a video URL lands in MainActivity, which forwards the intent here via
 * [setPendingFromIntent]. The webview pulls it with `getSharedText` (take + clear),
 * so cold-start (onCreate) and warm-share (onNewIntent) both work even if the
 * JS side was not listening yet.
 *
 * Pure pull model on purpose: no JS event emission from Kotlin, the frontend
 * polls `get_pending_share` on boot, on focus, and on an interval.
 */
@TauriPlugin
class ShareIntentPlugin(private val activity: Activity) : Plugin(activity) {

    companion object {
        private const val TAG = "ShareIntentPlugin"
        private const val MAX_CHARS = 32_768

        @Volatile
        private var pending: String? = null

        /**
         * Called from MainActivity.onCreate / onNewIntent (same package, no import needed).
         * Accepts SEND (single) and SEND_MULTIPLE, text/plain only.
         */
        @JvmStatic
        fun setPendingFromIntent(intent: Intent?) {
            if (intent == null) return
            val action = intent.action ?: return
            try {
                when (action) {
                    Intent.ACTION_SEND -> {
                        val text = intent.getCharSequenceExtra(Intent.EXTRA_TEXT)?.toString()
                        val subject = intent.getCharSequenceExtra(Intent.EXTRA_SUBJECT)?.toString()
                        val data = intent.dataString
                        val combined = listOfNotNull(
                            subject?.takeIf { it.isNotBlank() },
                            text?.takeIf { it.isNotBlank() },
                            data?.takeIf { it.isNotBlank() && (text == null || !text.contains(it)) },
                        ).joinToString("\n").trim()
                        if (combined.isNotBlank()) {
                            setPending(combined)
                        }
                    }
                    Intent.ACTION_SEND_MULTIPLE -> {
                        val texts = readMultipleTexts(intent)
                        val combined = texts.joinToString("\n").trim()
                        if (combined.isNotBlank()) {
                            setPending(combined)
                        }
                    }
                    else -> return
                }
            } catch (e: Exception) {
                Log.w(TAG, "setPendingFromIntent failed", e)
            }
        }

        private fun readMultipleTexts(intent: Intent): List<String> {
            val out = mutableListOf<String>()
            try {
                intent.getStringArrayListExtra(Intent.EXTRA_TEXT)?.let { out.addAll(it) }
            } catch (_: Exception) {
            }
            if (out.isEmpty()) {
                try {
                    @Suppress("DEPRECATION")
                    val seq = intent.getCharSequenceArrayListExtra(Intent.EXTRA_TEXT)
                    seq?.forEach { out.add(it.toString()) }
                } catch (_: Exception) {
                }
            }
            if (out.isEmpty()) {
                intent.getCharSequenceExtra(Intent.EXTRA_TEXT)?.toString()?.let {
                    if (it.isNotBlank()) out.add(it)
                }
            }
            return out.map { it.trim() }.filter { it.isNotBlank() }
        }

        @Synchronized
        private fun setPending(text: String) {
            val trimmed = text.trim().take(MAX_CHARS)
            if (trimmed.isBlank()) return
            pending = if (pending.isNullOrBlank()) {
                trimmed
            } else {
                // Append follow-up shares instead of dropping the earlier one.
                (pending!!.trimEnd() + "\n" + trimmed).take(MAX_CHARS)
            }
            Log.i(TAG, "pending share stored (${pending!!.length} chars)")
        }

        @Synchronized
        private fun takePending(): String? {
            val current = pending?.trim()?.takeIf { it.isNotBlank() }
            pending = null
            return current
        }
    }

    /**
     * Take + clear the pending shared text. Returns { text: string } ("" when none).
     */
    @Command
    fun getSharedText(invoke: Invoke) {
        val ret = JSObject()
        ret.put("text", takePending() ?: "")
        invoke.resolve(ret)
    }

    @Command
    fun clearSharedText(invoke: Invoke) {
        takePending()
        val ret = JSObject()
        ret.put("ok", true)
        invoke.resolve(ret)
    }
}
