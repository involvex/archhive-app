package com.archhive.app

import android.content.Context
import android.content.Intent
import androidx.work.Constraints
import androidx.work.ExistingWorkPolicy
import androidx.work.NetworkType
import androidx.work.OneTimeWorkRequestBuilder
import androidx.work.WorkManager
import androidx.work.Worker
import androidx.work.WorkerParameters

/**
 * Wakes the main activity when network is available so Rust can re-queue Pending /
 * WaitingForWifi downloads after process death. Does not run yt-dlp itself.
 */
class PendingResumeWorker(
    appContext: Context,
    params: WorkerParameters,
) : Worker(appContext, params) {

    override fun doWork(): Result {
        val launch = applicationContext.packageManager
            .getLaunchIntentForPackage(applicationContext.packageName)
            ?: return Result.success()
        launch.addFlags(Intent.FLAG_ACTIVITY_NEW_TASK)
        launch.putExtra(EXTRA_RESUME_DOWNLOADS, true)
        applicationContext.startActivity(launch)
        return Result.success()
    }

    companion object {
        const val UNIQUE_WORK = "archhive_pending_resume"
        const val EXTRA_RESUME_DOWNLOADS = "archhive.resume_downloads"

        fun schedule(context: Context, requireUnmetered: Boolean) {
            val network = if (requireUnmetered) {
                NetworkType.UNMETERED
            } else {
                NetworkType.CONNECTED
            }
            val constraints = Constraints.Builder()
                .setRequiredNetworkType(network)
                .build()
            val request = OneTimeWorkRequestBuilder<PendingResumeWorker>()
                .setConstraints(constraints)
                .build()
            WorkManager.getInstance(context).enqueueUniqueWork(
                UNIQUE_WORK,
                ExistingWorkPolicy.REPLACE,
                request,
            )
        }

        fun cancel(context: Context) {
            WorkManager.getInstance(context).cancelUniqueWork(UNIQUE_WORK)
        }
    }
}
