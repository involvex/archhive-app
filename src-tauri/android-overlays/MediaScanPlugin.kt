package com.archhive.app

import android.app.Activity
import android.media.MediaScannerConnection
import android.util.Log
import app.tauri.annotation.Command
import app.tauri.annotation.InvokeArg
import app.tauri.annotation.TauriPlugin
import app.tauri.plugin.Invoke
import app.tauri.plugin.Plugin

/**
 * MediaStore bridge for the media vault.
 *
 * Vaulting a scene *moves* the file out of the shared volume. Android's
 * MediaStore keeps the old row until something rescans that path, so gallery
 * and file apps can keep showing a thumbnail of a file that no longer exists
 * there. [scanFile] calls [MediaScannerConnection.scanFile], which drops the
 * stale row for a missing path and indexes a restored one.
 *
 * Called from Rust via `mobile::media_scan::rescan_paths`; failures are
 * tolerated by the caller (a stale gallery thumbnail is cosmetic).
 */
@TauriPlugin
class MediaScanPlugin(private val activity: Activity) : Plugin(activity) {

    @InvokeArg
    class ScanArgs {
        var paths: List<String> = emptyList()
    }

    companion object {
        private const val TAG = "MediaScanPlugin"
    }

    @Command
    fun scanFile(invoke: Invoke) {
        val args = try {
            invoke.parseArgs(ScanArgs::class.java)
        } catch (e: Exception) {
            Log.w(TAG, "scanFile: bad args: ${e.message}")
            invoke.reject("bad args: ${e.message}")
            return
        }

        val paths = args.paths.filter { it.isNotBlank() }.toTypedArray()
        if (paths.isEmpty()) {
            invoke.resolve()
            return
        }

        try {
            MediaScannerConnection.scanFile(
                activity.applicationContext,
                paths,
                null
            ) { path, uri ->
                // A missing file reports a null uri - that is the desired result
                // for vaulted media, so it is not an error.
                Log.d(TAG, "scanned $path -> $uri")
            }
            invoke.resolve()
        } catch (e: Exception) {
            Log.w(TAG, "scanFile failed: ${e.message}")
            invoke.reject("scan failed: ${e.message}")
        }
    }
}
