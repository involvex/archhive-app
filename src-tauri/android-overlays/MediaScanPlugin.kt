package com.archhive.app

import android.app.Activity
import android.content.ContentUris
import android.media.MediaScannerConnection
import android.os.Build
import android.provider.MediaStore
import android.util.Log
import app.tauri.annotation.Command
import app.tauri.annotation.InvokeArg
import app.tauri.annotation.TauriPlugin
import app.tauri.plugin.Invoke
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
 * [deleteFile] tries to delete via MediaStore/ContentResolver as a fallback
 * when direct file deletion fails due to scoped storage restrictions.
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
            invoke.resolve()
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
            invoke.resolve()
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
            invoke.resolve()
            return
        }

        val contentResolver = activity.applicationContext.contentResolver
        for (path in paths) {
            try {
                val file = File(path)
                // Try direct deletion first
                var deleted = file.delete()
                
                // If direct deletion failed, try aggressive deletion
                if (!deleted) {
                    try {
                        file.setWritable(true)
                        file.setReadable(true)
                        deleted = file.delete()
                    } catch (e: Exception) {
                        Log.w(TAG, "Permission change failed for $path: ${e.message}")
                    }
                }
                
                // Try query-based deletion to find URIs
                if (!deleted) {
                    try {
                        val projection = arrayOf(MediaStore.MediaColumns._ID, MediaStore.MediaColumns.DATA)
                        val uris = arrayOf(
                            MediaStore.Files.getContentUri("external"),
                            MediaStore.Video.Media.EXTERNAL_CONTENT_URI,
                            MediaStore.Images.Media.EXTERNAL_CONTENT_URI,
                            MediaStore.Audio.Media.EXTERNAL_CONTENT_URI
                        )
                        for (uri in uris) {
                            try {
                                contentResolver.query(uri, projection, "${MediaStore.MediaColumns.DATA} = ?", arrayOf(path), null)?.use { cursor ->
                                    val urisToDelete = mutableListOf<android.net.Uri>()
                                    while (cursor.moveToNext()) {
                                        val id = cursor.getLong(cursor.getColumnIndexOrThrow(MediaStore.MediaColumns._ID))
                                        val contentUri = android.content.ContentUris.withAppendedId(uri, id)
                                        urisToDelete.add(contentUri)
                                    }
                                    // Try direct delete first
                                    for (contentUri in urisToDelete) {
                                        try {
                                            val count = contentResolver.delete(contentUri, null, null)
                                            if (count > 0) {
                                                deleted = true
                                            }
                                        } catch (e: Exception) {
                                            Log.w(TAG, "ContentUri delete failed: ${e.message}")
                                        }
                                    }
                                    // If failed on Android 11+, try createDeleteRequest
                                    if (!deleted && urisToDelete.isNotEmpty() && android.os.Build.VERSION.SDK_INT >= android.os.Build.VERSION_CODES.R) {
                                        try {
                                            val request = android.provider.MediaStore.createDeleteRequest(contentResolver, urisToDelete)
                                            val intentSender = request.intentSender
                                            activity.startIntentSenderForResult(intentSender, 1001, null, 0, 0, 0)
                                            Log.i(TAG, "Requested user permission to delete $path via system dialog")
                                        } catch (e: Exception) {
                                            Log.w(TAG, "createDeleteRequest failed: ${e.message}")
                                        }
                                    }
                                }
                                if (deleted) break
                            } catch (e: Exception) {
                                Log.w(TAG, "Query for $uri failed: ${e.message}")
                            }
                        }
                    } catch (e: Exception) {
                        Log.w(TAG, "Query-based delete failed for $path: ${e.message}")
                    }
                }
                
                // Try MediaStore delete by DATA column as last resort
                if (!deleted) {
                    try {
                        val uris = arrayOf(
                            MediaStore.Files.getContentUri("external"),
                            MediaStore.Video.Media.EXTERNAL_CONTENT_URI
                        )
                        for (uri in uris) {
                            try {
                                val where = "${MediaStore.MediaColumns.DATA} = ?"
                                val selectionArgs = arrayOf(path)
                                val count = contentResolver.delete(uri, where, selectionArgs)
                                if (count > 0) {
                                    deleted = true
                                    break
                                }
                            } catch (e: Exception) {
                                Log.w(TAG, "MediaStore delete by DATA failed for $uri: ${e.message}")
                            }
                        }
                    } catch (e: Exception) {
                        Log.w(TAG, "MediaStore delete failed for $path: ${e.message}")
                    }
                }
                
                // Force scan to update MediaStore
                MediaScannerConnection.scanFile(activity.applicationContext, arrayOf(path), null) { scannedPath, uri ->
                    Log.d(TAG, "scanned after delete $scannedPath -> $uri")
                }
                
                if (!deleted && file.exists()) {
                    Log.w(TAG, "Could not delete $path, file still exists - may need MANAGE_EXTERNAL_STORAGE or user permission")
                } else if (deleted || !file.exists()) {
                    Log.d(TAG, "Successfully handled deletion for $path")
                }
            } catch (e: Exception) {
                Log.w(TAG, "deleteFile failed for $path: ${e.message}")
            }
        }
        invoke.resolve()
    }
}
