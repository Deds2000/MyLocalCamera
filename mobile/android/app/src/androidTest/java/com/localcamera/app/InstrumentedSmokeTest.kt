package com.localcamera.app

import androidx.test.platform.app.InstrumentationRegistry
import androidx.test.ext.junit.runners.AndroidJUnit4
import org.junit.Assert.assertEquals
import org.junit.Test
import org.junit.runner.RunWith

@RunWith(AndroidJUnit4::class)
class InstrumentedSmokeTest {
    @Test
    fun packageNameIsCorrect() {
        val ctx = InstrumentationRegistry.getInstrumentation().targetContext
        assertEquals("com.localcamera.app", ctx.packageName)
    }
}
