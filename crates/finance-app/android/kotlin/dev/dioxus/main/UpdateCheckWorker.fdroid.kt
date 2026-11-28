package dev.dioxus.main

import android.content.Context
import androidx.work.Worker
import androidx.work.WorkerParameters

// Variante F-Droid: sin auto-actualización ni WorkManager real.
class UpdateCheckWorker(appContext: Context, params: WorkerParameters) : Worker(appContext, params) {
    override fun doWork(): Result = Result.success()

    companion object {
        fun schedule(context: Context) {}
    }
}
