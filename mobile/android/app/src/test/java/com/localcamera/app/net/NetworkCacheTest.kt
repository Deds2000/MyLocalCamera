package com.localcamera.app.net

import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertSame
import org.junit.Assert.assertTrue
import org.junit.Test

class NetworkCacheTest {
    @Test
    fun availableNetworkReplacesStaleCacheAndLateLossDoesNotClearNewNetwork() {
        val cache = NetworkCache<Any>()
        val stale = Any()
        val restored = Any()

        assertTrue(cache.onAvailable(stale))
        assertTrue(cache.onAvailable(restored))
        assertSame(restored, cache.get())

        assertFalse(cache.onLost(stale))
        assertSame(restored, cache.get())
        assertTrue(cache.onLost(restored))
        assertNull(cache.get())
    }
}
