package com.archhive.app

import android.app.Activity
import android.content.ComponentName
import android.content.Intent
import android.content.pm.PackageManager
import android.util.Log
import app.tauri.annotation.Command
import app.tauri.annotation.TauriPlugin
import app.tauri.plugin.Invoke
import app.tauri.plugin.JSObject
import app.tauri.plugin.Plugin

/**
 * Termux bridge (RUN_COMMAND-Intent).
 *
 * ArcHive can hand work to the user's custom Termux fork over the standard Termux
 * external-API: an explicit intent to `com.invapp.app.RunCommandService` with the
 * RUN_COMMAND action + extras, executed by the Termux "app-shell" runner (no terminal
 * window needed). Used to push the encrypted-vault cookie jar into the Termux home so
 * the user's manual `yt-dlp`-in-Termux workflow has the same sessions as ArcHive.
 *
 * Constants are hardcoded from the fork source (`I:\dev\termux-app`), where the build
 * remaps everything to the runtime package `com.involvex.termux_app`:
 *   - package:    com.involvex.termux_app          (app/build.gradle applicationId)
 *   - service:    com.invapp.app.RunCommandService (Java namespace stays com.invapp)
 *   - action:     com.involvex.termux_app.RUN_COMMAND
 *   - permission: com.involvex.termux_app.permission.RUN_COMMAND
 *   - extras:     com.involvex.termux_app.RUN_COMMAND_{PATH,ARGUMENTS,WORKDIR,STDIN,RUNNER,LABEL}
 *
 * Requirements on the Termux side (both user-visible, surfaced as errors here):
 *   1. ArcHive must hold the fork's RUN_COMMAND permission (declared in ArcHive's
 *      manifest by `scripts/patch-android-cookie.ps1`; normal protection level, so it
 *      is auto-granted at install when Termux is present).
 *   2. Termux must have `allow-external-apps = true` in `~/.termux/termux.properties`,
 *      otherwise it shows a notification and drops the command.
 */
@TauriPlugin
class TermuxBridgePlugin(private val activity: Activity) : Plugin(activity) {

    companion object {
        private const val TAG = "TermuxBridgePlugin"

        /** Runtime package of the user's custom Termux fork. */
        const val TERMUX_PACKAGE = "com.involvex.termux_app"

        /** Service class name (Java namespace), not the runtime package. */
        const val RUN_COMMAND_SERVICE_CLASS = "com.invapp.app.RunCommandService"

        const val ACTION_RUN_COMMAND = "$TERMUX_PACKAGE.RUN_COMMAND"
        const val PERMISSION_RUN_COMMAND = "$TERMUX_PACKAGE.permission.RUN_COMMAND"

        private const val EXTRA_COMMAND_PATH = "$TERMUX_PACKAGE.RUN_COMMAND_PATH"
        private const val EXTRA_ARGUMENTS = "$TERMUX_PACKAGE.RUN_COMMAND_ARGUMENTS"
        private const val EXTRA_WORKDIR = "$TERMUX_PACKAGE.RUN_COMMAND_WORKDIR"
        private const val EXTRA_STDIN = "$TERMUX_PACKAGE.RUN_COMMAND_STDIN"
        private const val EXTRA_RUNNER = "$TERMUX_PACKAGE.RUN_COMMAND_RUNNER"
        private const val EXTRA_COMMAND_LABEL = "$TERMUX_PACKAGE.RUN_COMMAND_COMMAND_LABEL"

        /** Runner value for headless execution (ExecutionCommand.Runner.APP_SHELL). */
        private const val RUNNER_APP_SHELL = "app-shell"
    }

    private fun isTermuxInstalled(): Boolean = try {
        activity.packageManager.getPackageInfo(TERMUX_PACKAGE, 0)
        true
    } catch (_: PackageManager.NameNotFoundException) {
        false
    }

    private fun hasRunCommandPermission(): Boolean = try {
        activity.checkSelfPermission(PERMISSION_RUN_COMMAND) == PackageManager.PERMISSION_GRANTED
    } catch (_: Exception) {
        false
    }

    /**
     * Report whether the Termux bridge can be used. Returns
     * { installed: bool, permissionGranted: bool, available: bool, message: string }.
     */
    @Command
    fun status(invoke: Invoke) {
        val installed = isTermuxInstalled()
        val granted = hasRunCommandPermission()
        val ret = JSObject()
        ret.put("installed", installed)
        ret.put("permissionGranted", granted)
        ret.put("available", installed && granted)
        ret.put(
            "message",
            when {
                !installed -> "Termux ($TERMUX_PACKAGE) is not installed."
                !granted ->
                    "ArcHive is missing the Termux RUN_COMMAND permission. Reinstall ArcHive " +
                        "while Termux is installed, or grant it in Android Settings → Apps → ArcHive."
                else -> "Termux bridge ready."
            },
        )
        invoke.resolve(ret)
    }

    /**
     * Command payload for `sendRunCommand`. Plain class with a JVM no-arg
     * constructor + setters so Jackson (no Kotlin module on the Tauri side) can
     * bind { executable, arguments, stdin, workdir, label }.
     */
    class RunCommandArgs @JvmOverloads constructor(
        var executable: String = "",
        var arguments: Array<String> = emptyArray(),
        var stdin: String? = null,
        var workdir: String? = null,
        var label: String? = null,
    )

    /**
     * Send a RUN_COMMAND intent to the Termux fork.
     *
     * Payload: { executable: string, arguments: string[], stdin?: string,
     *            workdir?: string, label?: string }
     */
    @Command
    fun sendRunCommand(invoke: Invoke) {
        val payload = invoke.parseArgs(RunCommandArgs::class.java)
        val executable = payload.executable.trim()
        if (executable.isBlank()) {
            invoke.reject("sendRunCommand requires an executable path")
            return
        }
        if (!isTermuxInstalled()) {
            invoke.reject("Termux ($TERMUX_PACKAGE) is not installed.")
            return
        }
        if (!hasRunCommandPermission()) {
            invoke.reject(
                "ArcHive does not hold $PERMISSION_RUN_COMMAND. Reinstall ArcHive while " +
                    "Termux is installed, then retry.",
            )
            return
        }

        val args = ArrayList<String>()
        payload.arguments.forEach { args.add(it) }

        try {
            val intent = Intent(ACTION_RUN_COMMAND).apply {
                component = ComponentName(TERMUX_PACKAGE, RUN_COMMAND_SERVICE_CLASS)
                putExtra(EXTRA_COMMAND_PATH, executable)
                putStringArrayListExtra(EXTRA_ARGUMENTS, args)
                putExtra(EXTRA_RUNNER, RUNNER_APP_SHELL)
                putExtra(EXTRA_COMMAND_LABEL, payload.label ?: "ArcHive cookie handoff")
                payload.workdir?.takeIf { it.isNotBlank() }?.let {
                    putExtra(EXTRA_WORKDIR, it)
                }
                payload.stdin?.takeIf { it.isNotBlank() }?.let {
                    putExtra(EXTRA_STDIN, it)
                }
            }
            activity.startService(intent)
            val ret = JSObject()
            ret.put("ok", true)
            invoke.resolve(ret)
        } catch (e: SecurityException) {
            Log.w(TAG, "RUN_COMMAND denied", e)
            invoke.reject(
                "Termux refused the command (missing RUN_COMMAND permission). " +
                    "Grant it in Android Settings → Apps → ArcHive → Permissions.",
            )
        } catch (e: Exception) {
            Log.w(TAG, "sendRunCommand failed", e)
            invoke.reject("Could not reach Termux RUN_COMMAND service: ${e.message}")
        }
    }
}
