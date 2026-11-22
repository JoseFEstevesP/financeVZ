package dev.dioxus.main

import android.graphics.Color
import android.graphics.drawable.ColorDrawable
import android.os.Bundle
import androidx.core.view.WindowInsetsControllerCompat

typealias BuildConfig = com.gato99.financevz.BuildConfig

class MainActivity : WryActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)

        @Suppress("DEPRECATION")
        window.statusBarColor = Color.parseColor("#0f1420")
        @Suppress("DEPRECATION")
        window.navigationBarColor = Color.parseColor("#121826")
        window.setBackgroundDrawable(ColorDrawable(Color.parseColor("#0f1420")))

        WindowInsetsControllerCompat(window, window.decorView).apply {
            isAppearanceLightStatusBars = false
            isAppearanceLightNavigationBars = false
        }
    }
}
