package com.localcamera.app

import android.app.NotificationManager
import android.content.Context
import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.platform.app.InstrumentationRegistry
import com.localcamera.app.service.StreamingService
import com.localcamera.app.store.PairingStore
import org.junit.Assume.assumeTrue
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test
import org.junit.runner.RunWith

/** Verifies force-stop cleanup and removes only the pairing created for this test. */
@RunWith(AndroidJUnit4::class)
class ForceStopCleanupTest {
    @Test
    fun notificationIsGoneAndTestPairingCanBeForgotten() {
        val instrumentation = InstrumentationRegistry.getInstrumentation()
        val context = instrumentation.targetContext.applicationContext
        val pcId = InstrumentationRegistry.getArguments().getString(ARG_TEST_PC_ID)
        assumeTrue("test PC id is supplied only by the force-stop harness", pcId != null)
        val testPcId = requireNotNull(pcId)
        assertTrue("test PC id was invalid", UUID_PATTERN.matches(testPcId))

        val manager = context.getSystemService(Context.NOTIFICATION_SERVICE) as NotificationManager
        val notification = manager.activeNotifications
            .firstOrNull { it.id == StreamingService.NOTIF_ID }
        assertNull("foreground notification survived force-stop", notification)

        val store = PairingStore(context)
        store.forgetPc(testPcId)
        val unexpectedKey = store.deviceKey(testPcId)
        unexpectedKey?.fill(0)
        assertNull("force-stop test pairing key was not removed", unexpectedKey)
    }

    private companion object {
        const val ARG_TEST_PC_ID = "testPcId"
        val UUID_PATTERN = Regex("^[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[1-8][0-9a-fA-F]{3}-[89abAB][0-9a-fA-F]{3}-[0-9a-fA-F]{12}$")
    }
}
