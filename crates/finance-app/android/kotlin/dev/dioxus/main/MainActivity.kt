package dev.dioxus.main

import android.Manifest
import android.app.AlertDialog
import android.app.DownloadManager
import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent
import android.content.IntentFilter
import android.content.pm.PackageManager
import android.graphics.Color
import android.graphics.drawable.ColorDrawable
import android.net.Uri
import android.os.Build
import android.os.Bundle
import android.os.Environment
import android.webkit.JavascriptInterface
import android.webkit.WebView
import androidx.biometric.BiometricManager
import androidx.biometric.BiometricPrompt
import androidx.core.app.ActivityCompat
import androidx.core.content.ContextCompat
import androidx.core.view.WindowInsetsControllerCompat
import org.json.JSONObject
import java.io.File
import java.io.FileInputStream
import java.security.MessageDigest

typealias BuildConfig = com.gato99.financevz.BuildConfig

class MainActivity : WryActivity() {
    private var webView: WebView? = null
    private val prefs by lazy { getSharedPreferences("finanzasvz", MODE_PRIVATE) }

    companion object {
        private const val REQUEST_POST_NOTIFICATIONS = 2201
        private const val PREF_NOTIFICATION_PROMPT_DONE = "notification_prompt_done"
    }

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        applyTheme(prefs.getBoolean("light_theme", false))
        restorePendingDownload()
        requestNotificationPermissionIfNeeded()
        UpdateCheckWorker.schedule(this)
        handleOpenSettingsIntent(intent)
    }

    override fun onNewIntent(intent: Intent) {
        super.onNewIntent(intent)
        setIntent(intent)
        handleOpenSettingsIntent(intent)
    }

    override fun onResume() {
        super.onResume()
        checkPendingDownload()
        cleanupInstalledUpdate()
    }

    private fun cleanupInstalledUpdate() {
        try {
            val updatesDir = File(filesDir, "updates")
            val versionFile = File(updatesDir, "version-code.txt")
            val apkFile = File(updatesDir, "financevz.apk")
            val cachedVersion = if (versionFile.exists()) {
                versionFile.readText().trim().toLongOrNull()
            } else null
            val installedVersion = if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.P) {
                packageManager.getPackageInfo(packageName, 0).longVersionCode
            } else {
                @Suppress("DEPRECATION")
                packageManager.getPackageInfo(packageName, 0).versionCode.toLong()
            }
            if (cachedVersion != null && cachedVersion <= installedVersion) {
                apkFile.delete()
                versionFile.delete()
            }
            val external = getExternalFilesDir(Environment.DIRECTORY_DOWNLOADS)
            if (cachedVersion != null && cachedVersion <= installedVersion) {
                external?.listFiles()
                    ?.filter { it.isFile && it.name.startsWith("financevz-update") && it.name.endsWith(".apk") }
                    ?.forEach { it.delete() }
            }
        } catch (_: Exception) {}
    }

    private fun restorePendingDownload() {
        pendingDownloadId = prefs.getLong("pending_download_id", -1L)
        pendingSha256 = prefs.getString("pending_download_sha", "") ?: ""
        pendingVersionCode = prefs.getLong("pending_download_version", 0L)
        if (pendingDownloadId != -1L) {
            registerDownloadReceiver()
            checkPendingDownload()
        }
    }

    private fun checkPendingDownload() {
        val id = pendingDownloadId
        if (id == -1L) return
        val dm = getSystemService(Context.DOWNLOAD_SERVICE) as DownloadManager
        val cursor = dm.query(DownloadManager.Query().setFilterById(id))
        cursor.use {
            if (!it.moveToFirst()) return
            val statusIdx = it.getColumnIndex(DownloadManager.COLUMN_STATUS)
            val status = if (statusIdx >= 0) it.getInt(statusIdx) else return
            if (status == DownloadManager.STATUS_SUCCESSFUL || status == DownloadManager.STATUS_FAILED) {
                handleDownloadComplete(id)
            } else {
                notifyDownload("started", "Descarga en segundo plano en curso…")
            }
        }
    }

    override fun onWebViewCreate(view: WebView) {
        webView = view
        view.settings.textZoom = 100
        view.addJavascriptInterface(FinanzasVZBridge(), "FinanzasVZBiometric")
        if (intent?.getBooleanExtra("open_settings", false) == true) {
            view.postDelayed({ notifyOpenSettings() }, 700)
        }
    }

    private fun handleOpenSettingsIntent(intent: Intent?) {
        if (intent?.getBooleanExtra("open_settings", false) == true) {
            webView?.postDelayed({ notifyOpenSettings() }, 300)
        }
    }

    private fun notifyOpenSettings() {
        webView?.post {
            val payload = JSONObject().put("kind", "open_settings")
            webView?.evaluateJavascript("window.__fvzBridge && window.__fvzBridge.send($payload)", null)
        }
    }

    private fun requestNotificationPermissionIfNeeded() {
        if (Build.VERSION.SDK_INT < Build.VERSION_CODES.TIRAMISU) return
        if (ContextCompat.checkSelfPermission(this, Manifest.permission.POST_NOTIFICATIONS) == PackageManager.PERMISSION_GRANTED) return
        if (prefs.getBoolean(PREF_NOTIFICATION_PROMPT_DONE, false)) return
        showPermissionRationale()
    }

    private fun showPermissionRationale() {
        AlertDialog.Builder(this)
            .setTitle("Avisos de actualización")
            .setMessage("finanzasVZ solo envía avisos cuando publica una versión nueva; nunca envía datos ni publicidad. Sin este permiso la app sigue funcionando, pero no podrá avisarte.")
            .setPositiveButton("Permitir") { _, _ ->
                ActivityCompat.requestPermissions(
                    this,
                    arrayOf(Manifest.permission.POST_NOTIFICATIONS),
                    REQUEST_POST_NOTIFICATIONS
                )
            }
            .setNegativeButton("Ahora no") { _, _ ->
                prefs.edit().putBoolean(PREF_NOTIFICATION_PROMPT_DONE, true).apply()
            }
            .show()
    }

    override fun onRequestPermissionsResult(
        requestCode: Int,
        permissions: Array<out String>,
        grantResults: IntArray
    ) {
        super.onRequestPermissionsResult(requestCode, permissions, grantResults)
        if (requestCode != REQUEST_POST_NOTIFICATIONS) return
        prefs.edit().putBoolean(PREF_NOTIFICATION_PROMPT_DONE, true).apply()
        if (grantResults.firstOrNull() == PackageManager.PERMISSION_GRANTED) return
        AlertDialog.Builder(this)
            .setTitle("Avisos desactivados")
            .setMessage("No recibirás avisos de nuevas versiones. Puedes activarlos en Ajustes del sistema → Notificaciones → finanzasVZ.")
            .setPositiveButton("Abrir ajustes") { _, _ -> openNotificationSettings() }
            .setNegativeButton("Cerrar", null)
            .show()
    }

    private fun openNotificationSettings() {
        val intent = if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
            Intent(android.provider.Settings.ACTION_APP_NOTIFICATION_SETTINGS)
                .putExtra(android.provider.Settings.EXTRA_APP_PACKAGE, packageName)
        } else {
            Intent(android.provider.Settings.ACTION_APPLICATION_DETAILS_SETTINGS)
                .setData(Uri.parse("package:$packageName"))
        }
        try {
            startActivity(intent)
        } catch (_: Exception) {
        }
    }

    private fun applyTheme(light: Boolean) {
        val background = if (light) "#f7f9fd" else "#0f1420"
        val navigation = if (light) "#eef2fb" else "#121826"
        @Suppress("DEPRECATION")
        window.statusBarColor = Color.parseColor(background)
        @Suppress("DEPRECATION")
        window.navigationBarColor = Color.parseColor(navigation)
        window.setBackgroundDrawable(ColorDrawable(Color.parseColor(background)))
        WindowInsetsControllerCompat(window, window.decorView).apply {
            isAppearanceLightStatusBars = light
            isAppearanceLightNavigationBars = light
        }
    }

    inner class FinanzasVZBridge {
        @JavascriptInterface
        fun authenticate(action: String) {
            runOnUiThread { showBiometricPrompt(action) }
        }

        @JavascriptInterface
        fun setLightTheme(light: Boolean) {
            prefs.edit().putBoolean("light_theme", light).apply()
            runOnUiThread { applyTheme(light) }
        }

        @JavascriptInterface
        fun downloadUpdate(url: String, sha256: String, versionCode: Long, versionName: String) {
            runOnUiThread { enqueueDownload(url, sha256, versionCode, versionName) }
        }
    }

    private var pendingDownloadId: Long = -1L
    private var pendingSha256: String = ""
    private var pendingVersionCode: Long = 0L
    private var downloadReceiver: BroadcastReceiver? = null

    private fun enqueueDownload(url: String, sha256: String, versionCode: Long, versionName: String) {
        if (pendingDownloadId != -1L) {
            notifyDownload("started", "Ya hay una descarga en curso. Espera a que termine.")
            return
        }
        try {
            val updatesDir = File(filesDir, "updates")
            updatesDir.mkdirs()
            val destFile = File(updatesDir, "financevz.apk")
            if (destFile.exists()) destFile.delete()
            val versionFile = File(updatesDir, "version-code.txt")
            if (versionFile.exists()) versionFile.delete()

            val external = getExternalFilesDir(Environment.DIRECTORY_DOWNLOADS)
            external?.mkdirs()
            val externalFile = File(external, "financevz-update.apk")
            if (externalFile.exists()) externalFile.delete()

            val request = DownloadManager.Request(Uri.parse(url))
                .setTitle("financeVZ $versionName")
                .setDescription("Descargando actualización en segundo plano…")
                .setNotificationVisibility(DownloadManager.Request.VISIBILITY_VISIBLE_NOTIFY_COMPLETED)
                .setMimeType("application/vnd.android.package-archive")
                .setDestinationUri(Uri.fromFile(externalFile))
                .setAllowedOverMetered(true)
                .setAllowedOverRoaming(true)

            val dm = getSystemService(Context.DOWNLOAD_SERVICE) as DownloadManager
            pendingSha256 = sha256
            pendingVersionCode = versionCode
            pendingDownloadId = dm.enqueue(request)
            prefs.edit()
                .putLong("pending_download_id", pendingDownloadId)
                .putString("pending_download_sha", sha256)
                .putLong("pending_download_version", versionCode)
                .apply()
            registerDownloadReceiver()
            notifyDownload("started", "Descarga iniciada en segundo plano. Puedes cerrar la app.")
        } catch (e: Exception) {
            notifyDownload("error", "No se pudo iniciar la descarga: ${e.message}")
        }
    }

    private fun registerDownloadReceiver() {
        if (downloadReceiver != null) return
        downloadReceiver = object : BroadcastReceiver() {
            override fun onReceive(context: Context?, intent: Intent?) {
                val id = intent?.getLongExtra(DownloadManager.EXTRA_DOWNLOAD_ID, -1L) ?: return
                if (id != pendingDownloadId) return
                handleDownloadComplete(id)
            }
        }
        val filter = IntentFilter(DownloadManager.ACTION_DOWNLOAD_COMPLETE)
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU) {
            registerReceiver(downloadReceiver, filter, Context.RECEIVER_EXPORTED)
        } else {
            @Suppress("UnspecifiedRegisterReceiverFlag")
            registerReceiver(downloadReceiver, filter)
        }
    }

    private fun handleDownloadComplete(id: Long) {
        val dm = getSystemService(Context.DOWNLOAD_SERVICE) as DownloadManager
        val query = DownloadManager.Query().setFilterById(id)
        val cursor = dm.query(query)
        var localUri: String? = null
        cursor.use {
            if (!it.moveToFirst()) {
                notifyDownload("error", "No se pudo verificar la descarga.")
                return
            }
            val statusIdx = it.getColumnIndex(DownloadManager.COLUMN_STATUS)
            val status = if (statusIdx >= 0) it.getInt(statusIdx) else -1
            if (status != DownloadManager.STATUS_SUCCESSFUL) {
                notifyDownload("error", "La descarga falló. Intenta de nuevo.")
                cleanupFailedDownload()
                return
            }
            val uriIdx = it.getColumnIndex(DownloadManager.COLUMN_LOCAL_URI)
            if (uriIdx >= 0) localUri = it.getString(uriIdx)
        }
        val external = getExternalFilesDir(Environment.DIRECTORY_DOWNLOADS)
        val externalFile = resolveDownloadedApk(external, localUri)
        if (externalFile == null || !externalFile.exists() || externalFile.length() < 1_000_000L) {
            notifyDownload("error", "El APK descargado no es válido.")
            cleanupFailedDownload()
            return
        }
        if (pendingSha256.isNotBlank()) {
            val actual = sha256Hex(externalFile)
            if (!actual.equals(pendingSha256, ignoreCase = true)) {
                externalFile.delete()
                notifyDownload("error", "El hash SHA-256 del APK no coincide.")
                cleanupFailedDownload()
                return
            }
        }
        val updatesDir = File(filesDir, "updates")
        updatesDir.mkdirs()
        val apk = File(updatesDir, "financevz.apk")
        if (apk.exists()) apk.delete()
        externalFile.copyTo(apk, overwrite = true)
        externalFile.delete()
        File(updatesDir, "version-code.txt").writeText(pendingVersionCode.toString())
        prefs.edit().remove("pending_download_id").remove("pending_download_sha").remove("pending_download_version").apply()
        notifyDownload("completed", "Actualización descargada. Pulsa «Instalar APK descargado».")
    }

    private fun resolveDownloadedApk(external: File?, localUri: String?): File? {
        if (localUri != null) {
            try {
                val parsed = Uri.parse(localUri)
                if (parsed.scheme == "file") {
                    val path = parsed.path
                    if (path != null) {
                        val file = File(path)
                        if (file.exists()) return file
                    }
                }
            } catch (_: Exception) {}
        }
        val preferred = File(external, "financevz-update.apk")
        if (preferred.exists()) return preferred
        val dir = external ?: return null
        return dir.listFiles()
            ?.filter { it.isFile && it.name.startsWith("financevz-update") && it.name.endsWith(".apk") }
            ?.maxByOrNull { it.lastModified() }
    }

    private fun cleanupFailedDownload() {
        val updatesDir = File(filesDir, "updates")
        File(updatesDir, "financevz.apk").delete()
        File(updatesDir, "version-code.txt").delete()
        val external = getExternalFilesDir(Environment.DIRECTORY_DOWNLOADS)
        File(external, "financevz-update.apk").delete()
        external?.listFiles()
            ?.filter { it.isFile && it.name.startsWith("financevz-update") && it.name.endsWith(".apk") }
            ?.forEach { it.delete() }
        prefs.edit().remove("pending_download_id").remove("pending_download_sha").remove("pending_download_version").apply()
    }

    private fun sha256Hex(file: File): String {
        val md = MessageDigest.getInstance("SHA-256")
        FileInputStream(file).use { input ->
            val buf = ByteArray(64 * 1024)
            while (true) {
                val read = input.read(buf)
                if (read <= 0) break
                md.update(buf, 0, read)
            }
        }
        return md.digest().joinToString("") { "%02x".format(it) }
    }

    private fun notifyDownload(status: String, message: String) {
        webView?.post {
            val payload = JSONObject()
                .put("kind", "download_result")
                .put("status", status)
                .put("message", message)
            webView?.evaluateJavascript("window.__fvzBridge && window.__fvzBridge.send($payload)", null)
        }
    }

    override fun onDestroy() {
        downloadReceiver?.let {
            try { unregisterReceiver(it) } catch (_: Exception) {}
        }
        downloadReceiver = null
        super.onDestroy()
    }

    private fun showBiometricPrompt(action: String) {
        val available = BiometricManager.from(this).canAuthenticate(
            BiometricManager.Authenticators.BIOMETRIC_STRONG
        )
        if (available != BiometricManager.BIOMETRIC_SUCCESS) {
            notifyBiometric(action, false, "No hay una huella configurada en este teléfono.")
            return
        }
        val callback = object : BiometricPrompt.AuthenticationCallback() {
            override fun onAuthenticationSucceeded(result: BiometricPrompt.AuthenticationResult) {
                notifyBiometric(action, true, "")
            }

            override fun onAuthenticationError(errorCode: Int, errString: CharSequence) {
                if (errorCode == BiometricPrompt.ERROR_NEGATIVE_BUTTON ||
                    errorCode == BiometricPrompt.ERROR_USER_CANCELED
                ) return
                notifyBiometric(action, false, errString.toString())
            }
        }
        val prompt = BiometricPrompt(this, ContextCompat.getMainExecutor(this), callback)
        val subtitle = if (action == "enroll") {
            "Confirma tu huella para guardarla"
        } else {
            "Confirma tu huella para entrar"
        }
        val info = BiometricPrompt.PromptInfo.Builder()
            .setTitle("finanzasVZ")
            .setSubtitle(subtitle)
            .setNegativeButtonText("Usar PIN")
            .build()
        prompt.authenticate(info)
    }

    private fun notifyBiometric(action: String, ok: Boolean, message: String) {
        webView?.post {
            val payload = JSONObject()
                .put("kind", "biometric_result")
                .put("action", action)
                .put("ok", ok)
                .put("message", message)
            webView?.evaluateJavascript("window.__fvzBridge.send($payload)", null)
        }
    }
}
