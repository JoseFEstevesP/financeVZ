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
        applyTheme(prefs.getBoolean("light_theme", false))
    }

    override fun onWebViewCreate(view: WebView) {
        webView = view
        view.settings.textZoom = 100
        view.addJavascriptInterface(FinanzasVZBridge(), "FinanzasVZBiometric")
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
            runOnUiThread {
                prefs.edit().putBoolean("light_theme", light).apply()
                applyTheme(light)
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
}
