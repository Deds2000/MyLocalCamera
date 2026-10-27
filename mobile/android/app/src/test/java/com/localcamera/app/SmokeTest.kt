package com.localcamera.app

import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch
import kotlinx.coroutines.test.advanceTimeBy
import kotlinx.coroutines.test.runCurrent
import kotlinx.coroutines.test.runTest
import org.junit.Assert.assertEquals
import org.junit.Test

@OptIn(ExperimentalCoroutinesApi::class)
class SmokeTest {
    @Test
    fun virtualTimeWorks() = runTest {
        var ticks = 0
        launch { repeat(3) { delay(1000); ticks++ } }
        advanceTimeBy(2500); runCurrent()
        assertEquals(2, ticks)
    }
}
