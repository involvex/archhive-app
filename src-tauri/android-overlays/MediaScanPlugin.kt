package com.archhive.app

import android.app.Activity
import android.content.ContentUris
import android.media.MediaScannerConnection
import android.net.Uri
import android.os.Build
import android.provider.MediaStore
import android.util.Log
import androidx.activity.result.ActivityResult
import androidx.activity.result.IntentSenderRequest
import app.tauri.annotation.ActivityCallback
import app.tauri.annotation.Command
import app.tauri.annotation.InvokeArg
import app.tauri.annotation.TauriPlugin
import app.tauri.plugin.Invoke
import app.tauri.plugin.JSArray
import app.tauri.plugin.JSObject
import app.tauri.plugin.Plugin
import java.io.File

/**
 * MediaStore bridge for the media vault.
 *
 * Vaulting a scene *moves* the file out of the shared volume. Android's
 * MediaStore keeps the old row until something tells it to rescan that path, so gallery
 * and file apps can keep showing a thumbnail of a file that no longer exists
 * there. [scanFile] calls [MediaScannerConnection.scanFile], which drops the
 * stale row for a missing path and indexes a restored one.
 *
 * [deleteFile] tries filesystem delete, ContentResolver delete by matched URI,
 * then [MediaStore.createDeleteRequest] (API 30+) with a proper activity-result
 * callback when user consent is required.
 */
@TauriPlugin
class MediaScanPlugin(private val activity: Activity) : Plugin(activity) {

    @InvokeArg
    class ScanArgs {
        var paths: List<String> = emptyList()
    }

    @InvokeArg
    class DeleteArgs {
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
            invoke.resolve(emptyResult(deleted = true, pending = false))
            return
        }

        try {
            MediaScannerConnection.scanFile(
                activity.applicationContext,
                paths,
                null
            ) { path, uri ->
                Log.d(TAG, "scanned $path -> $uri")
            }
            invoke.resolve(emptyResult(deleted = true, pending = false))
        } catch (e: Exception) {
            Log.w(TAG, "scanFile failed: ${e.message}")
            invoke.reject("scan failed: ${e.message}")
        }
    }

    @Command
    fun deleteFile(invoke: Invoke) {
        val args = try {
            invoke.parseArgs(DeleteArgs::class.java)
        } catch (e: Exception) {
            Log.w(TAG, "deleteFile: bad args: ${e.message}")
            invoke.reject("bad args: ${e.message}")
            return
        }

        val paths = args.paths.filter { it.isNotBlank() }
        if (paths.isEmpty()) {
            invoke.resolve(emptyResult(deleted = true, pending = false))
            return
        }

        val contentResolver = activity.applicationContext.contentResolver
        val leftoverPaths = mutableListOf<String>()
        val consentUris = linkedSetOf<Uri>()
        var anyDeleted = false

        for (path in paths) {
            try {
                val file = File(path)
                var deleted = tryDeleteFile(file)

                if (!deleted) {
                    val uris = findMediaUris(path)
                    for (contentUri in uris) {
                        try {
                            val count = contentResolver.delete(contentUri, null, null)
                            if (count > 0) {
                                deleted = true
                                anyDeleted = true
                            } else if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.R) {
                                consentUris.add(contentUri)
                            }
                        } catch (e: SecurityException) {
                            Log.w(TAG, "ContentUri delete needs consent for $contentUri: ${e.message}")
                            if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.R) {
                                consentUris.add(contentUri)
                            }
                        } catch (e: Exception) {
                            Log.w(TAG, "ContentUri delete failed: ${e.message}")
                            if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.R) {
                                consentUris.add(contentUri)
                            }
                        }
                    }
                } else {
                    anyDeleted = true
                }

                // Only scan missing paths — scanning an existing leftover re-indexes it.
                if (deleted || !file.exists()) {
                    MediaScannerConnection.scanFile(
                        activity.applicationContext,
                        arrayOf(path),
                        null
                    ) { scannedPath, uri ->
                        Log.d(TAG, "scanned after delete $scannedPath -> $uri")
                    }
                }

                if (!deleted && file.exists()) {
                    leftoverPaths.add(path)
                    Log.w(TAG, "Could not delete $path without user consent / elevated permission")
                }
            } catch (e: Exception) {
                Log.w(TAG, "deleteFile failed for $path: ${e.message}")
                leftoverPaths.add(path)
            }
        }

        if (consentUris.isNotEmpty() && Build.VERSION.SDK_INT >= Build.VERSION_CODES.R) {
            try {
                val request = MediaStore.createDeleteRequest(contentResolver, consentUris.toList())
                val intentSenderRequest = IntentSenderRequest.Builder(request.intentSender).build()
                Log.i(TAG, "Requesting user permission to delete ${consentUris.size} MediaStore row(s)")
                startIntentSenderForResult(invoke, intentSenderRequest, "onDeleteRequestResult")
                return
            } catch (e: Exception) {
                Log.w(TAG, "createDeleteRequest failed: ${e.message}")
            }
        }

        invoke.resolve(
            deleteResult(
                deleted = leftoverPaths.isEmpty(),
                pendingUserConsent = false,
                leftoverPaths = leftoverPaths
            )
        )
    }

    @ActivityCallback
    fun onDeleteRequestResult(invoke: Invoke, result: ActivityResult) {
        val approved = result.resultCode == Activity.RESULT_OK
        Log.i(TAG, "createDeleteRequest resultCode=${result.resultCode} approved=$approved")
        invoke.resolve(
            deleteResult(
                deleted = approved,
                pendingUserConsent = !approved,
                leftoverPaths = emptyList()
            )
        )
    }

    private fun tryDeleteFile(file: File): Boolean {
        if (!file.exists()) {
            return true
        }
        if (file.delete()) {
            return true
        }
        return try {
            file.setWritable(true)
            file.setReadable(true)
            file.delete()
        } catch (e: Exception) {
            Log.w(TAG, "Permission change failed for ${file.absolutePath}: ${e.message}")
            false
        }
    }

    /**
     * Find MediaStore URIs for [path]. Prefer DISPLAY_NAME (+ RELATIVE_PATH / SIZE)
     * because DATA is often null on scoped storage (API 29+).
     */
    private fun findMediaUris(path: String): List<Uri> {
        val file = File(path)
        val displayName = file.name
        val size = if (file.exists()) file.length() else -1L
        val relativePath = relativePathFromAbsolute(path)
        val collections = listOf(
            MediaStore.Files.getContentUri("external"),
            MediaStore.Video.Media.EXTERNAL_CONTENT_URI,
            MediaStore.Images.Media.EXTERNAL_CONTENT_URI,
            MediaStore.Audio.Media.EXTERNAL_CONTENT_URI
        )
        val found = linkedSetOf<Uri>()

        for (collection in collections) {
            queryAndCollect(
                collection,
                "${MediaStore.MediaColumns.DATA} = ?",
                arrayOf(path),
                found
            )

            if (!relativePath.isNullOrBlank()) {
                queryAndCollect(
                    collection,
                    "${MediaStore.MediaColumns.DISPLAY_NAME} = ? AND ${MediaStore.MediaColumns.RELATIVE_PATH} = ?",
                    arrayOf(displayName, relativePath),
                    found
                )
            }

            if (size >= 0) {
                queryAndCollect(
                    collection,
                    "${MediaStore.MediaColumns.DISPLAY_NAME} = ? AND ${MediaStore.MediaColumns.SIZE} = ?",
                    arrayOf(displayName, size.toString()),
                    found
                )
            }

            if (found.isEmpty()) {
                queryAndCollect(
                    collection,
                    "${MediaStore.MediaColumns.DISPLAY_NAME} = ?",
                    arrayOf(displayName),
                    found
                )
            }
        }
        return found.toList()
    }

    private fun queryAndCollect(
        collection: Uri,
        selection: String,
        args: Array<String>,
        out: MutableSet<Uri>
    ) {
        val contentResolver = activity.applicationContext.contentResolver
        try {
            val projection = arrayOf(MediaStore.MediaColumns._ID)
            contentResolver.query(collection, projection, selection, args, null)?.use { cursor ->
                val idIdx = cursor.getColumnIndexOrThrow(MediaStore.MediaColumns._ID)
                while (cursor.moveToNext()) {
                    val id = cursor.getLong(idIdx)
                    out.add(ContentUris.withAppendedId(collection, id))
                }
            }
        } catch (e: Exception) {
            Log.w(TAG, "Query failed for $collection ($selection): ${e.message}")
        }
    }

    /**
     * Derive MediaStore RELATIVE_PATH from an absolute path under primary storage.
     * Example: `/storage/emulated/0/Download/foo/bar.mp4` → `Download/foo/`
     */
    private fun relativePathFromAbsolute(path: String): String? {
        val normalized = path.replace('\\', '/')
        val markers = listOf(
            "/storage/emulated/0/",
            "/sdcard/",
            "/mnt/sdcard/"
        )
        for (marker in markers) {
            val idx = normalized.indexOf(marker, ignoreCase = true)
            if (idx >= 0) {
                val after = normalized.substring(idx + marker.length)
                val slash = after.lastIndexOf('/')
                if (slash <= 0) {
                    return ""
                }
                return after.substring(0, slash + 1)
            }
        }
        val publicFolders = listOf("Download/", "Downloads/", "DCIM/", "Pictures/", "Movies/", "Music/")
        for (folder in publicFolders) {
            val idx = normalized.indexOf("/$folder", ignoreCase = true)
            if (idx >= 0) {
                val fromFolder = normalized.substring(idx + 1)
                val slash = fromFolder.lastIndexOf('/')
                if (slash <= 0) return folder
                return fromFolder.substring(0, slash + 1)
            }
        }
        return null
    }

    private fun emptyResult(deleted: Boolean, pending: Boolean): JSObject {
        return deleteResult(deleted, pending, emptyList())
    }

    private fun deleteResult(
        deleted: Boolean,
        pendingUserConsent: Boolean,
        leftoverPaths: List<String>
    ): JSObject {
        val ret = JSObject()
        ret.put("deleted", deleted)
        ret.put("pendingUserConsent", pendingUserConsent)
        val arr = JSArray()
        for (p in leftoverPaths) {
            arr.put(p)
        }
        ret.put("leftoverPaths", arr)
        return ret
    }
}
