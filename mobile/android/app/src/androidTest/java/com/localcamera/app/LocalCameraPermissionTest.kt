package com.localcamera.app

import android.Manifest
import android.content.Intent
import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.platform.app.InstrumentationRegistry
import androidx.test.rule.GrantPermissionRule
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith
import java.util.concurrent.CountDownLatch
import java.util.concurrent.TimeUnit
import java.util.concurrent.atomic.AtomicReference

@RunWith(AndroidJUnit4::class)
class LocalCameraPermissionTest {
    @get:Rule
    val grantCamera = GrantPermissionRule.grant(Manifest.permission.CAMERA)

    @Test
    fun pluginReportsAndRequestsTheActualCameraPermission() {
        val instrumentation = InstrumentationRegistry.getInstrumentation()
        val context = instrumentation.targetContext
        val activity = instrumentation.startActivitySync(
            Intent(Intent.ACTION_MAIN)
                .setClassName(context.packageName, MainActivity::class.java.name)
                .addFlags(Intent.FLAG_ACTIVITY_NEW_TASK),
        ) as MainActivity

        try {
            val startCheck = evaluate(
                activity,
                "window.Capacitor.Plugins.LocalCamera.getPermissions().then(p => " +
                    "window.__lcCameraPermission = p.camera); true",
            )
            assertEquals("true", startCheck)
            assertTrue(awaitJavascript(activity) { "window.__lcCameraPermission === 'granted'" })

            val startRequest = evaluate(
                activity,
                "window.Capacitor.Plugins.LocalCamera.requestPermission({kind:'camera'})" +
                    ".then(p => window.__lcRequestedPermission = p.status); true",
            )
            assertEquals("true", startRequest)
            assertTrue(awaitJavascript(activity) { "window.__lcRequestedPermission === 'granted'" })
        } finally {
            evaluate(activity, "delete window.__lcCameraPermission; delete window.__lcRequestedPermission; true")
            instrumentation.runOnMainSync { activity.finish() }
        }
    }

    private fun awaitJavascript(activity: MainActivity, expression: () -> String): Boolean {
        val deadline = System.currentTimeMillis() + 5_000L
        while (System.currentTimeMillis() < deadline) {
            if (evaluate(activity, expression()) == "true") return true
            Thread.sleep(50L)
        }
        return false
    }

    private fun evaluate(activity: MainActivity, script: String): String? {
        val instrumentation = InstrumentationRegistry.getInstrumentation()
        val result = AtomicReference<String?>()
        val latch = CountDownLatch(1)
        instrumentation.runOnMainSync {
            activity.bridge.webView.evaluateJavascript(script) { value ->
                result.set(value)
                latch.countDown()
            }
        }
        if (!latch.await(2, TimeUnit.SECONDS)) return null
        return result.get()?.trim('"')
    }
}
