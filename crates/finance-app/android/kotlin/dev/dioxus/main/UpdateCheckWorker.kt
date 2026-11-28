package dev.dioxus.main

import android.Manifest
import android.app.NotificationChannel
import android.app.NotificationManager
import android.app.PendingIntent
import android.content.Context
import android.content.Intent
import android.content.pm.PackageManager
import android.os.Build
import androidx.core.app.NotificationCompat
import androidx.core.app.NotificationManagerCompat
import androidx.core.content.ContextCompat
import androidx.work.Constraints
import androidx.work.ExistingPeriodicWorkPolicy
import androidx.work.NetworkType
import androidx.work.PeriodicWorkRequestBuilder
import androidx.work.WorkManager
import androidx.work.Worker
import androidx.work.WorkerParameters
import org.json.JSONObject
import java.net.HttpURLConnection
import java.net.URL
import java.util.concurrent.TimeUnit

class UpdateCheckWorker(appContext: Context, params: WorkerParameters) : Worker(appContext, params) {
    override fun doWork(): Result {
        return try {
            val prefs = applicationContext.getSharedPreferences("finanzasvz", Context.MODE_PRIVATE)
            val configured = prefs.getString("update_url", "")?.trim().orEmpty()
            val info = if (configured.isEmpty()) {
                fetchGithubRelease()
            } else {
                fetchLocalRelease(configured.trimEnd('/'))
            }
            if (info == null) return Result.success()

            val localCode = localVersionCode(applicationContext)
            val notified = prefs.getLong("notified_update_version", -1L)
            if (info.versionCode <= localCode) {
                cancelUpdateNotifications(applicationContext, notified)
                prefs.edit().remove("notified_update_version").apply()
                return Result.success()
            }

            if (notified == info.versionCode) return Result.success()

            showUpdateNotification(applicationContext, info.versionName, info.versionCode)
            if (notificationsEnabled(applicationContext)) {
                prefs.edit().putLong("notified_update_version", info.versionCode).apply()
            }
            Result.success()
        } catch (_: Exception) {
            Result.success()
        }
    }

    private data class UpdateInfo(val versionCode: Long, val versionName: String)

    private fun fetchGithubRelease(): UpdateInfo? {
        val conn = (URL("https://api.github.com/repos/JoseFEstevesP/financeVZ/releases/latest").openConnection() as HttpURLConnection)
        conn.setRequestProperty("User-Agent", "financeVZ")
        conn.setRequestProperty("Accept", "application/vnd.github+json")
        conn.connectTimeout = 15000
        conn.readTimeout = 15000
        conn.inputStream.use { stream ->
            val body = stream.bufferedReader().readText()
            val json = JSONObject(body)
            val tag = json.optString("tag_name", "")
            val (code, name) = versionFromTag(tag) ?: return null
            val assets = json.optJSONArray("assets") ?: return null
            var hasApk = false
            for (i in 0 until assets.length()) {
                val asset = assets.optJSONObject(i) ?: continue
                val assetName = asset.optString("name", "")
                if (assetName.lowercase().endsWith(".apk")) {
                    hasApk = true
                    break
                }
            }
            if (!hasApk) return null
            return UpdateInfo(code, name)
        }
    }

    private fun fetchLocalRelease(base: String): UpdateInfo? {
        val conn = (URL("$base/update.json").openConnection() as HttpURLConnection)
        conn.connectTimeout = 15000
        conn.readTimeout = 15000
        conn.inputStream.use { stream ->
            val body = stream.bufferedReader().readText()
            val json = JSONObject(body)
            val code = json.optLong("version_code", json.optLong("versionCode", -1L))
            val name = json.optString("version_name", json.optString("versionName", ""))
            if (code <= 0L || name.isBlank()) return null
            return UpdateInfo(code, name)
        }
    }

    private fun versionFromTag(tag: String): Pair<Long, String>? {
        val clean = tag.removePrefix("v").removePrefix("V")
        val parts = clean.split('.')
        if (parts.size != 3) return null
        val code = parts[2].toLongOrNull() ?: return null
        return code to clean
    }

    private fun localVersionCode(context: Context): Long {
        return try {
            val pkg = context.packageManager.getPackageInfo(context.packageName, 0)
            if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.P) pkg.longVersionCode else @Suppress("DEPRECATION") pkg.versionCode.toLong()
        } catch (_: Exception) {
            -1L
        }
    }

    private fun notificationsEnabled(context: Context): Boolean {
        if (Build.VERSION.SDK_INT < Build.VERSION_CODES.TIRAMISU) return true
        return ContextCompat.checkSelfPermission(
            context,
            Manifest.permission.POST_NOTIFICATIONS
        ) == PackageManager.PERMISSION_GRANTED
    }

    private fun showUpdateNotification(context: Context, versionName: String, versionCode: Long) {
        if (!notificationsEnabled(context)) return
        val nm = context.getSystemService(Context.NOTIFICATION_SERVICE) as NotificationManager
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
            val channel = NotificationChannel(CHANNEL_ID, "Actualizaciones", NotificationManager.IMPORTANCE_DEFAULT)
            channel.description = "Avisos cuando hay una nueva versión de finanzasVZ"
            nm.createNotificationChannel(channel)
        }
        val intent = Intent(context, MainActivity::class.java).apply {
            flags = Intent.FLAG_ACTIVITY_NEW_TASK or Intent.FLAG_ACTIVITY_CLEAR_TOP
            putExtra("open_settings", true)
        }
        val pending = PendingIntent.getActivity(
            context,
            1001,
            intent,
            PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_IMMUTABLE
        )
        val notification = NotificationCompat.Builder(context, CHANNEL_ID)
            .setSmallIcon(android.R.drawable.stat_sys_download_done)
            .setContentTitle("Actualización disponible")
            .setContentText("finanzasVZ v$versionName está lista para descargar.")
            .setAutoCancel(true)
            .setContentIntent(pending)
            .build()
        NotificationManagerCompat.from(context).notify(versionCode.toInt(), notification)
    }

    private fun cancelUpdateNotifications(context: Context, notifiedVersion: Long) {
        val nm = NotificationManagerCompat.from(context)
        if (notifiedVersion > 0L) {
            nm.cancel(notifiedVersion.toInt())
        }
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.M) {
            val service = context.getSystemService(Context.NOTIFICATION_SERVICE) as NotificationManager
            service.activeNotifications
                .filter { it.packageName == context.packageName && it.notification.channelId == CHANNEL_ID }
                .forEach { nm.cancel(it.id) }
        }
    }

    companion object {
        private const val CHANNEL_ID = "financevz_updates"
        private const val WORK_NAME = "financevz-update-check"

        fun schedule(context: Context) {
            val constraints = Constraints.Builder()
                .setRequiredNetworkType(NetworkType.CONNECTED)
                .build()
            val request = PeriodicWorkRequestBuilder<UpdateCheckWorker>(6, TimeUnit.HOURS)
                .setConstraints(constraints)
                .build()
            WorkManager.getInstance(context).enqueueUniquePeriodicWork(
                WORK_NAME,
                ExistingPeriodicWorkPolicy.UPDATE,
                request
            )
        }
    }
}
