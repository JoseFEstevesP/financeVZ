package dev.dioxus.main

import android.graphics.Color
import android.graphics.drawable.ColorDrawable
import android.os.Bundle
import android.webkit.JavascriptInterface
import android.webkit.WebView
import androidx.biometric.BiometricManager
import androidx.biometric.BiometricPrompt
import androidx.core.content.ContextCompat
import androidx.core.view.WindowInsetsControllerCompat
import org.json.JSONObject

typealias BuildConfig = com.joseE.financevz.BuildConfig

class MainActivity : WryActivity() {
    private var webView: WebView? = null
    private val prefs by lazy { getSharedPreferences("finanzasvz", MODE_PRIVATE) }

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        applyTheme(prefs.getString("theme_id", if (prefs.getBoolean("light_theme", false)) "light" else "dark") ?: "dark")
    }

    private var pendingBackupJson: String? = null

    private val createBackupLauncher = registerForActivityResult(
        androidx.activity.result.contract.ActivityResultContracts.CreateDocument("application/json")
    ) { uri: Uri? ->
        val json = pendingBackupJson
        pendingBackupJson = null
        if (uri == null || json == null) return@registerForActivityResult
        try {
            contentResolver.openOutputStream(uri)?.use { out ->
                out.write(json.toByteArray(Charsets.UTF_8))
            }
            notifyBackupResult(true, "Copia exportada correctamente.")
        } catch (e: Exception) {
            notifyBackupResult(false, "No se pudo guardar la copia: ${e.message ?: "error"}")
        }
    }

    override fun onWebViewCreate(view: WebView) {
        webView = view
        view.settings.textZoom = 100
        view.addJavascriptInterface(FinanzasVZBridge(), "FinanzasVZBiometric")
    }

    private data class ThemePalette(val background: String, val navigation: String, val lightBars: Boolean)

    private fun themePalette(themeId: String): ThemePalette = when (themeId) {
        "light" -> ThemePalette("#f7f9fd", "#eef2fb", true)
        "oled" -> ThemePalette("#000000", "#000000", false)
        "sepia" -> ThemePalette("#f4ecd8", "#fff8e8", true)
        "high-contrast" -> ThemePalette("#000000", "#000000", false)
        "commit" -> ThemePalette("#0d1117", "#161b22", false)
        else -> ThemePalette("#0f1420", "#121826", false)
    }

    private fun applyTheme(themeId: String) {
        val palette = themePalette(themeId)
        @Suppress("DEPRECATION")
        window.statusBarColor = Color.parseColor(palette.background)
        @Suppress("DEPRECATION")
        window.navigationBarColor = Color.parseColor(palette.navigation)
        window.setBackgroundDrawable(ColorDrawable(Color.parseColor(palette.background)))
        WindowInsetsControllerCompat(window, window.decorView).apply {
            isAppearanceLightStatusBars = palette.lightBars
            isAppearanceLightNavigationBars = palette.lightBars
        }
    }

    inner class FinanzasVZBridge {
        @JavascriptInterface
        fun authenticate(action: String) {
            runOnUiThread { showBiometricPrompt(action) }
        }

        @JavascriptInterface
        fun setLightTheme(light: Boolean) {
            setTheme(if (light) "light" else "dark")
        }

        @JavascriptInterface
        fun setTheme(themeId: String) {
            runOnUiThread {
                val normalized = when (themeId) {
                    "light", "oled", "sepia", "high-contrast", "commit", "dark" -> themeId
                    else -> "dark"
                }
                prefs.edit().putString("theme_id", normalized).putBoolean("light_theme", normalized == "light" || normalized == "sepia").apply()
                applyTheme(normalized)
            }
        }

        @JavascriptInterface
        fun saveBackup(json: String) {
            runOnUiThread {
                pendingBackupJson = json
                val name = "financevz-backup-" +
                    java.text.SimpleDateFormat("yyyyMMdd-HHmmss", java.util.Locale.US)
                        .format(java.util.Date()) + ".json"
                try {
                    createBackupLauncher.launch(name)
                } catch (e: Exception) {
                    pendingBackupJson = null
                    notifyBackupResult(false, "No se pudo abrir el guardado: ${e.message ?: "error"}")
                }
            }
        }
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
            webView?.evaluateJavascript("window.__fvzBridge && window.__fvzBridge.send($payload)", null)
        }
    }

    private fun notifyBackupResult(ok: Boolean, message: String) {
        webView?.post {
            val payload = JSONObject()
                .put("kind", "backup_result")
                .put("ok", ok)
                .put("message", message)
            webView?.evaluateJavascript("window.__fvzBridge && window.__fvzBridge.send($payload)", null)
        }
    }
}
