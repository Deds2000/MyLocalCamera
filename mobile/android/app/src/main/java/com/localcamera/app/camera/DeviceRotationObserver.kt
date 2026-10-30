package com.localcamera.app.camera

import android.content.ComponentCallbacks2
import android.content.Context
import android.content.res.Configuration
import android.hardware.display.DisplayManager

/**
 * Observa a rotação do aparelho para o [CameraXBackend] (cartão F2-T07,
 * passo 3): um `ComponentCallbacks2` registrado no contexto dispara
 * `onConfigurationChanged` quando o aparelho gira; o backend recalcula o
 * hint na hora. Funciona sem Activity (a `StreamingService` do F2-T12 o
 * mantém vivo).
 */
internal class DeviceRotationObserver(
    private val context: Context,
    private val onConfigurationChanged: () -> Unit,
) {
    private val callback = object : ComponentCallbacks2 {
        override fun onConfigurationChanged(newConfig: Configuration) = onConfigurationChanged()

        // Ambos deprecated em API 14, mas exigidos pela interface
        @Suppress("OVERRIDE_DEPRECATION")
        override fun onLowMemory() = Unit

        @Suppress("OVERRIDE_DEPRECATION")
        override fun onTrimMemory(level: Int) = Unit
    }

    private var registered = false

    fun register() {
        if (registered) return
        context.registerComponentCallbacks(callback)
        registered = true
    }

    fun unregister() {
        if (!registered) return
        registered = false
        try {
            context.unregisterComponentCallbacks(callback)
        } catch (t: Throwable) {
            // ignora: o processo pode estar finalizando
        }
    }
}

/**
 * Rotação atual do display principal (0/90/180/270), em graus.
 * `id 0` = display distinto padrão (a constante `DisplayManager
 * .DISTINCT_DISPLAY_DEFAULT` não existe nos stubs do compileSdk 36 —
 * mesmo caso das chaves `SCALER_*` no F2-T06).
 */
internal fun currentDisplayRotation(ctx: Context): Int {
    val dm = ctx.getSystemService(Context.DISPLAY_SERVICE) as DisplayManager
    val display = dm.getDisplay(0) ?: return 0
    return display.rotation * 90
}
