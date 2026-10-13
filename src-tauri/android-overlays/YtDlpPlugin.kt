package com.archhive.app

import android.app.Activity
import android.util.Log
import app.tauri.annotation.Command
import app.tauri.annotation.TauriPlugin
import app.tauri.plugin.Invoke
import app.tauri.plugin.JSObject
import app.tauri.plugin.Plugin
import com.yausername.ffmpeg.FFmpeg
import com.yausername.youtubedl_android.YoutubeDL
import com.yausername.youtubedl_android.YoutubeDLRequest
import kotlinx.coroutines.CoroutineExceptionHandler
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import java.io.BufferedReader
import java.io.File
import java.io.InputStreamReader
import java.util.concurrent.TimeUnit

@TauriPlugin
class YtDlpPlugin(private val activity: Activity) : Plugin(activity) {

    // Last-resort guard: an uncaught Throwable in a plugin coroutine (e.g. a
    // LinkageError from youtubedl-android) must not take the whole app down.
    private val scope = CoroutineScope(
        Dispatchers.IO + CoroutineExceptionHandler { _, e ->
            Log.e("YtDlpPlugin", "Uncaught error in plugin coroutine", e)
        }
    )
    @Volatile private var initialized = false
    @Volatile private var ffmpegPath: String? = null
    @Volatile private var ffprobePath: String? = null
    @Volatile private var ffmpegLibDir: String? = null
    /** Last init failure, surfaced in "not initialized" responses for diagnosis. */
    @Volatile private var initError: String? = null

    /**
     * Lazily initialize YoutubeDL + FFmpeg (idempotent, thread-safe).
     * FFmpeg.init unpacks Android-native shared libs from libffmpeg.zip.so into
     * app-private storage. The executables are libffmpeg.so / libffprobe.so in
     * the native library dir and must be run with LD_LIBRARY_PATH set to the
     * unpacked packages/ffmpeg/usr/lib directory.
     */
    @Synchronized
    private fun ensureInitialized(): Boolean {
        if (initialized) return true
        return try {
            YoutubeDL.getInstance().init(activity)
            try {
                FFmpeg.getInstance().init(activity)
            } catch (e: Throwable) {
                // FFmpeg module may be absent in some builds; yt-dlp can still run.
                Log.w("YtDlpPlugin", "FFmpeg.init failed (media tools may be unavailable)", e)
            }
            waitForFfmpegLibsUnpack()
            resolveMediaToolsLocked()
            initialized = true
            true
        } catch (e: Throwable) {
            Log.e("YtDlpPlugin", "Failed to init YoutubeDL", e)
            initError = "${e.javaClass.simpleName}: ${e.message ?: ""}"
            false
        }
    }

    private fun notInitialized(what: String): String {
        val cause = initError ?: return "$what not initialized"
        return "$what not initialized ($cause)"
    }

    /** FFmpeg.init is async; wait until libavdevice appears in the unpacked tree. */
    private fun waitForFfmpegLibsUnpack() {
        val libDir = File(
            activity.applicationContext.noBackupFilesDir,
            "youtubedl-android/packages/ffmpeg/usr/lib"
        )
        val marker = File(libDir, "libavdevice.so.61")
        var waited = 0
        while (waited < 60_000 && (!libDir.isDirectory || !marker.isFile)) {
            Thread.sleep(250)
            waited += 250
        }
    }

    @Synchronized
    private fun resolveMediaToolsLocked() {
        val ctx = activity.applicationContext
        val nativeDir = File(ctx.applicationInfo.nativeLibraryDir)
        val ffmpegUsrLib = File(ctx.noBackupFilesDir, "youtubedl-android/packages/ffmpeg/usr/lib")
        // libc++_shared.so (needed by librubberband.so) ships in the python package.
        val pythonUsrLib = File(ctx.noBackupFilesDir, "youtubedl-android/packages/python/usr/lib")
        val libDirs = buildList {
            if (ffmpegUsrLib.isDirectory) add(ffmpegUsrLib.absolutePath)
            if (pythonUsrLib.isDirectory) add(pythonUsrLib.absolutePath)
            if (nativeDir.isDirectory) add(nativeDir.absolutePath)
        }
        ffmpegLibDir = libDirs.takeIf { it.isNotEmpty() }?.joinToString(":")

        val pkgRoot = File(ctx.noBackupFilesDir, "youtubedl-android/packages/ffmpeg")
        // Prefer unpacked ELF binaries (correct RUNPATH) over jni stub libffmpeg.so.
        ffmpegPath = findNamedFile(pkgRoot, "ffmpeg", maxDepth = 8)?.absolutePath
        ffprobePath = findNamedFile(pkgRoot, "ffprobe", maxDepth = 8)?.absolutePath

        if (ffmpegPath == null || ffprobePath == null) {
            val ff = File(nativeDir, "libffmpeg.so")
            val fp = File(nativeDir, "libffprobe.so")
            if (ffmpegPath == null && ff.isFile && ff.length() > 1024L) {
                ffmpegPath = ff.absolutePath
            }
            if (ffprobePath == null && fp.isFile && fp.length() > 1024L) {
                ffprobePath = fp.absolutePath
            }
        }

        Log.i(
            "YtDlpPlugin",
            "media tools: ffmpeg=${ffmpegPath ?: "missing"} ffprobe=${ffprobePath ?: "missing"} libDir=${ffmpegLibDir ?: "missing"}"
        )
    }

    private fun findNamedFile(dir: File, name: String, maxDepth: Int): File? {
        if (maxDepth < 0 || !dir.isDirectory) return null
        val children = dir.listFiles() ?: return null
        for (child in children) {
            // Exact binary name only — skip directories named "ffmpeg" and lib*.so stubs.
            if (child.isFile && child.name == name && child.length() > 1024L) {
                child.setExecutable(true, false)
                return child
            }
        }
        for (child in children) {
            if (child.isDirectory) {
                findNamedFile(child, name, maxDepth - 1)?.let { return it }
            }
        }
        return null
    }

    private fun shellQuote(value: String): String =
        "'" + value.replace("'", "'\\''") + "'"

    private fun runProcess(binary: String, args: Array<String>): Triple<String, String, Int> {
        resolveMediaToolsLocked()
        val libDir = ffmpegLibDir
        val argList = args.joinToString(" ") { shellQuote(it) }
        val cmd: Array<String> = if (!libDir.isNullOrBlank()) {
            val script =
                "export LD_LIBRARY_PATH=${shellQuote(libDir)}; exec ${shellQuote(binary)} $argList"
            arrayOf("/system/bin/sh", "-c", script)
        } else {
            arrayOf(binary, *args)
        }
        val pb = ProcessBuilder(*cmd)
            .redirectErrorStream(false)
            .directory(activity.filesDir)
        val proc = pb.start()
        val stdout = StringBuilder()
        val stderr = StringBuilder()
        val outReader = Thread {
            BufferedReader(InputStreamReader(proc.inputStream)).use { r ->
                var line: String?
                while (r.readLine().also { line = it } != null) {
                    stdout.append(line).append('\n')
                }
            }
        }
        val errReader = Thread {
            BufferedReader(InputStreamReader(proc.errorStream)).use { r ->
                var line: String?
                while (r.readLine().also { line = it } != null) {
                    stderr.append(line).append('\n')
                }
            }
        }
        outReader.start()
        errReader.start()
        val finished = proc.waitFor(10, TimeUnit.MINUTES)
        outReader.join(5_000)
        errReader.join(5_000)
        val code = if (finished) proc.exitValue() else {
            proc.destroyForcibly()
            -1
        }
        return Triple(stdout.toString(), stderr.toString(), code)
    }

    /**
     * Ensure FFmpeg/ffprobe are initialized and report paths/versions.
     * Returns: { ok, ffmpegPath, ffprobePath, ffmpegVersion, ffprobeVersion, message }
     */
    @Command
    fun ensureMediaTools(invoke: Invoke) {
        scope.launch {
            val ret = JSObject()
            if (!ensureInitialized()) {
                ret.put("ok", false)
                ret.put("message", notInitialized("YoutubeDL/FFmpeg"))
                invoke.resolve(ret)
                return@launch
            }
            // Re-resolve in case init just unpacked
            withContext(Dispatchers.IO) {
                resolveMediaToolsLocked()
            }
            val ff = ffmpegPath
            val fp = ffprobePath
            ret.put("ok", ff != null && fp != null)
            ret.put("ffmpegPath", ff ?: "")
            ret.put("ffprobePath", fp ?: "")
            ret.put("message", if (ff != null && fp != null) "ok" else "ffmpeg/ffprobe binaries not found after FFmpeg.init")
            var ffmpegOk = false
            var ffprobeOk = false
            if (ff != null) {
                val (out, err, code) = withContext(Dispatchers.IO) {
                    runProcess(ff, arrayOf("-version"))
                }
                val ver = (out.ifBlank { err }).lineSequence().firstOrNull()?.trim() ?: ""
                ret.put("ffmpegVersion", ver)
                ffmpegOk = code == 0 && ver.contains("ffmpeg", ignoreCase = true)
            } else {
                ret.put("ffmpegVersion", "")
            }
            if (fp != null) {
                val (out, err, code) = withContext(Dispatchers.IO) {
                    runProcess(fp, arrayOf("-version"))
                }
                val ver = (out.ifBlank { err }).lineSequence().firstOrNull()?.trim() ?: ""
                ret.put("ffprobeVersion", ver)
                ffprobeOk = code == 0 && ver.contains("ffprobe", ignoreCase = true)
            } else {
                ret.put("ffprobeVersion", "")
            }
            ret.put("ok", ffmpegOk && ffprobeOk)
            if (!(ffmpegOk && ffprobeOk)) {
                ret.put(
                    "message",
                    "ffmpeg/ffprobe present but failed -version (check LD_LIBRARY_PATH / FFmpeg.init unpack)"
                )
            }
            invoke.resolve(ret)
        }
    }

    /**
     * Execute ffmpeg or ffprobe.
     * Args: { tool: "ffmpeg"|"ffprobe", args: string[] }
     * Returns: { stdout, stderr, exitCode }
     */
    @Command
    fun executeMedia(invoke: Invoke) {
        val payload = invoke.parseArgs(MediaArgs::class.java)
        val tool = payload.tool.trim().lowercase()
        if (tool != "ffmpeg" && tool != "ffprobe") {
            val ret = JSObject()
            ret.put("stdout", "")
            ret.put("stderr", "tool must be ffmpeg or ffprobe")
            ret.put("exitCode", -1)
            invoke.resolve(ret)
            return
        }

        scope.launch {
            if (!ensureInitialized()) {
                val ret = JSObject()
                ret.put("stdout", "")
                ret.put("stderr", notInitialized("YoutubeDL/FFmpeg"))
                ret.put("exitCode", -1)
                invoke.resolve(ret)
                return@launch
            }
            val binary = if (tool == "ffmpeg") ffmpegPath else ffprobePath
            if (binary == null) {
                val ret = JSObject()
                ret.put("stdout", "")
                ret.put("stderr", "$tool binary not found (FFmpeg.init did not unpack it)")
                ret.put("exitCode", -1)
                invoke.resolve(ret)
                return@launch
            }
            try {
                val (stdout, stderr, code) = withContext(Dispatchers.IO) {
                    runProcess(binary, payload.args)
                }
                val ret = JSObject()
                ret.put("stdout", stdout)
                ret.put("stderr", stderr)
                ret.put("exitCode", code)
                invoke.resolve(ret)
            } catch (e: Throwable) {
                val ret = JSObject()
                ret.put("stdout", "")
                ret.put("stderr", e.message ?: "Unknown error")
                ret.put("exitCode", -1)
                invoke.resolve(ret)
            }
        }
    }

    /**
     * Execute a yt-dlp command.
     * Args: { args: string[] }
     * Returns: { stdout: string, stderr: string, exitCode: number }
     */
    @Command
    fun execute(invoke: Invoke) {
        val args = invoke.parseArgs(Args::class.java)

        if (!ensureInitialized()) {
            val ret = JSObject()
            ret.put("stdout", "")
            ret.put("stderr", notInitialized("YoutubeDL"))
            ret.put("exitCode", -1)
            invoke.resolve(ret)
            return
        }

        scope.launch {
            try {
                val request = YoutubeDLRequest(args.args.toList())
                val result = withContext(Dispatchers.IO) {
                    YoutubeDL.getInstance().execute(request)
                }
                val ret = JSObject()
                ret.put("stdout", result.out)
                ret.put("stderr", result.err)
                ret.put("exitCode", 0)
                invoke.resolve(ret)
            } catch (e: Throwable) {
                val ret = JSObject()
                ret.put("stdout", "")
                ret.put("stderr", e.message ?: "Unknown error")
                ret.put("exitCode", -1)
                invoke.resolve(ret)
            }
        }
    }

    /**
     * Get yt-dlp version.
     * Returns: { version: string }
     */
    @Command
    fun version(invoke: Invoke) {
        if (!ensureInitialized()) {
            val ret = JSObject()
            ret.put("version", "")
            invoke.resolve(ret)
            return
        }

        scope.launch {
            try {
                val ver = withContext(Dispatchers.IO) {
                    YoutubeDL.getInstance().version(activity)
                }
                val ret = JSObject()
                ret.put("version", ver ?: "")
                invoke.resolve(ret)
            } catch (e: Throwable) {
                val ret = JSObject()
                ret.put("version", "")
                invoke.resolve(ret)
            }
        }
    }

    /**
     * Update yt-dlp to latest version.
     * Returns: { success: boolean, message: string }
     */
    @Command
    fun update(invoke: Invoke) {
        if (!ensureInitialized()) {
            val ret = JSObject()
            ret.put("success", false)
            ret.put("message", notInitialized("YoutubeDL"))
            invoke.resolve(ret)
            return
        }

        scope.launch {
            try {
                withContext(Dispatchers.IO) {
                    YoutubeDL.getInstance().updateYoutubeDL(activity)
                }
                val ret = JSObject()
                ret.put("success", true)
                ret.put("message", "Updated successfully")
                invoke.resolve(ret)
            } catch (e: Throwable) {
                val ret = JSObject()
                ret.put("success", false)
                ret.put("message", e.message ?: "Update failed")
                invoke.resolve(ret)
            }
        }
    }

    /** Start / refresh foreground keep-alive while downloads are Active. */
    @Command
    fun startKeepAlive(invoke: Invoke) {
        val args = invoke.parseArgs(KeepAliveArgs::class.java)
        try {
            DownloadForegroundService.start(
                activity.applicationContext,
                args.title.ifBlank { "ArcHive downloads" },
                args.text.ifBlank { "Downloading…" },
                args.progress,
            )
            val ret = JSObject()
            ret.put("ok", true)
            invoke.resolve(ret)
        } catch (e: Throwable) {
            val ret = JSObject()
            ret.put("ok", false)
            ret.put("message", e.message ?: "startKeepAlive failed")
            invoke.resolve(ret)
        }
    }

    @Command
    fun updateKeepAlive(invoke: Invoke) {
        val args = invoke.parseArgs(KeepAliveArgs::class.java)
        try {
            DownloadForegroundService.update(
                activity.applicationContext,
                args.title.ifBlank { "ArcHive downloads" },
                args.text.ifBlank { "Downloading…" },
                args.progress,
            )
            val ret = JSObject()
            ret.put("ok", true)
            invoke.resolve(ret)
        } catch (e: Throwable) {
            val ret = JSObject()
            ret.put("ok", false)
            ret.put("message", e.message ?: "updateKeepAlive failed")
            invoke.resolve(ret)
        }
    }

    @Command
    fun stopKeepAlive(invoke: Invoke) {
        try {
            DownloadForegroundService.stop(activity.applicationContext)
            val ret = JSObject()
            ret.put("ok", true)
            invoke.resolve(ret)
        } catch (e: Throwable) {
            val ret = JSObject()
            ret.put("ok", false)
            ret.put("message", e.message ?: "stopKeepAlive failed")
            invoke.resolve(ret)
        }
    }

    /**
     * Open a local file in another app (video player, gallery, …) via
     * ACTION_VIEW. Android forbids file:// URIs across apps, so the file is
     * shared through the app's FileProvider (`${applicationId}.fileprovider`).
     */
    @Command
    fun openFile(invoke: Invoke) {
        val args = invoke.parseArgs(OpenFileArgs::class.java)
        val ret = JSObject()
        try {
            val file = File(args.path)
            if (!file.isFile) {
                ret.put("ok", false)
                ret.put("message", "File not found: ${args.path}")
                invoke.resolve(ret)
                return
            }
            val uri = androidx.core.content.FileProvider.getUriForFile(
                activity,
                "${activity.packageName}.fileprovider",
                file,
            )
            val mime = args.mime.ifBlank {
                android.webkit.MimeTypeMap.getSingleton()
                    .getMimeTypeFromExtension(file.extension.lowercase()) ?: "video/*"
            }
            val intent = android.content.Intent(android.content.Intent.ACTION_VIEW)
                .setDataAndType(uri, mime)
                .addFlags(android.content.Intent.FLAG_GRANT_READ_URI_PERMISSION)
            activity.startActivity(android.content.Intent.createChooser(intent, "Open with"))
            ret.put("ok", true)
        } catch (e: Throwable) {
            Log.w("YtDlpPlugin", "openFile failed for ${args.path}", e)
            ret.put("ok", false)
            ret.put("message", e.message ?: e.javaClass.simpleName)
        }
        invoke.resolve(ret)
    }

    /**
     * Video fullscreen. wry's WebChromeClient cancels HTML5 fullscreen
     * (onShowCustomView), so the web player expands itself and asks for an
     * immersive window here: system bars hidden (swipe to peek) and, for
     * landscape video, a sensor-landscape orientation lock.
     */
    /**
     * Whether ArcHive may manage all files in shared storage ("All files
     * access", Android 11+). Needed to delete library files in e.g.
     * Download/ArcHive that this install didn't create. Android 10 and
     * older use WRITE_EXTERNAL_STORAGE instead and report granted.
     */
    @Command
    fun storageAccess(invoke: Invoke) {
        val ret = JSObject()
        val granted = if (android.os.Build.VERSION.SDK_INT >= android.os.Build.VERSION_CODES.R) {
            android.os.Environment.isExternalStorageManager()
        } else {
            true
        }
        ret.put("ok", granted)
        ret.put("message", if (granted) "granted" else "missing")
        invoke.resolve(ret)
    }

    /** Open the system "All files access" screen for ArcHive. */
    @Command
    fun openStorageAccessSettings(invoke: Invoke) {
        activity.runOnUiThread {
            val ret = JSObject()
            try {
                if (android.os.Build.VERSION.SDK_INT >= android.os.Build.VERSION_CODES.R) {
                    val intent = try {
                        android.content.Intent(
                            android.provider.Settings.ACTION_MANAGE_APP_ALL_FILES_ACCESS_PERMISSION,
                            android.net.Uri.parse("package:" + activity.packageName),
                        )
                    } catch (e: Throwable) {
                        android.content.Intent(
                            android.provider.Settings.ACTION_MANAGE_ALL_FILES_ACCESS_PERMISSION,
                        )
                    }
                    try {
                        activity.startActivity(intent)
                    } catch (e: android.content.ActivityNotFoundException) {
                        activity.startActivity(
                            android.content.Intent(
                                android.provider.Settings.ACTION_MANAGE_ALL_FILES_ACCESS_PERMISSION,
                            ),
                        )
                    }
                }
                ret.put("ok", true)
            } catch (e: Throwable) {
                Log.w("YtDlpPlugin", "openStorageAccessSettings failed", e)
                ret.put("ok", false)
                ret.put("message", e.message ?: e.javaClass.simpleName)
            }
            invoke.resolve(ret)
        }
    }

    @Command
    fun setImmersive(invoke: Invoke) {
        val args = invoke.parseArgs(ImmersiveArgs::class.java)
        activity.runOnUiThread {
            val ret = JSObject()
            try {
                val window = activity.window
                val controller = androidx.core.view.WindowCompat.getInsetsController(
                    window,
                    window.decorView,
                )
                if (args.enabled) {
                    controller.systemBarsBehavior =
                        androidx.core.view.WindowInsetsControllerCompat
                            .BEHAVIOR_SHOW_TRANSIENT_BARS_BY_SWIPE
                    controller.hide(androidx.core.view.WindowInsetsCompat.Type.systemBars())
                    activity.requestedOrientation = if (args.landscape) {
                        android.content.pm.ActivityInfo.SCREEN_ORIENTATION_SENSOR_LANDSCAPE
                    } else {
                        android.content.pm.ActivityInfo.SCREEN_ORIENTATION_UNSPECIFIED
                    }
                } else {
                    controller.show(androidx.core.view.WindowInsetsCompat.Type.systemBars())
                    activity.requestedOrientation =
                        android.content.pm.ActivityInfo.SCREEN_ORIENTATION_UNSPECIFIED
                }
                ret.put("ok", true)
            } catch (e: Throwable) {
                Log.w("YtDlpPlugin", "setImmersive failed", e)
                ret.put("ok", false)
                ret.put("message", e.message ?: e.javaClass.simpleName)
            }
            invoke.resolve(ret)
        }
    }

    /** Schedule WorkManager to wake the app when network is available. */
    @Command
    fun schedulePendingResume(invoke: Invoke) {
        val args = invoke.parseArgs(ResumeArgs::class.java)
        try {
            PendingResumeWorker.schedule(activity.applicationContext, args.requireUnmetered)
            val ret = JSObject()
            ret.put("ok", true)
            invoke.resolve(ret)
        } catch (e: Throwable) {
            val ret = JSObject()
            ret.put("ok", false)
            ret.put("message", e.message ?: "schedulePendingResume failed")
            invoke.resolve(ret)
        }
    }

    @Command
    fun cancelPendingResume(invoke: Invoke) {
        try {
            PendingResumeWorker.cancel(activity.applicationContext)
            val ret = JSObject()
            ret.put("ok", true)
            invoke.resolve(ret)
        } catch (e: Throwable) {
            val ret = JSObject()
            ret.put("ok", false)
            ret.put("message", e.message ?: "cancelPendingResume failed")
            invoke.resolve(ret)
        }
    }

    /**
     * Command payload. Plain class with a JVM no-arg constructor + setter so
     * Jackson (no Kotlin module on the Tauri side) can bind {"args": [...]}.
     */
    class Args @JvmOverloads constructor(var args: Array<String> = emptyArray())

    class MediaArgs @JvmOverloads constructor(
        var tool: String = "ffmpeg",
        var args: Array<String> = emptyArray(),
    )

    class KeepAliveArgs @JvmOverloads constructor(
        var title: String = "ArcHive downloads",
        var text: String = "Downloading…",
        var progress: Int = -1,
    )

    class ResumeArgs @JvmOverloads constructor(
        var requireUnmetered: Boolean = false,
    )

    class ImmersiveArgs @JvmOverloads constructor(
        var enabled: Boolean = false,
        var landscape: Boolean = true,
    )

    class OpenFileArgs @JvmOverloads constructor(
        var path: String = "",
        var mime: String = "",
    )
}
