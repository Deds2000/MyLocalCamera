package com.localcamera.app

import android.Manifest
import android.content.Intent
import android.net.Uri
import android.os.ParcelFileDescriptor
import android.util.Base64
import android.util.Log
import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.platform.app.InstrumentationRegistry
import androidx.test.rule.GrantPermissionRule
import com.getcapacitor.BridgeActivity
import com.localcamera.app.service.AppForeground
import com.localcamera.app.service.StreamingService
import com.localcamera.app.store.PairingStore
import org.junit.Assert.assertTrue
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith
import java.net.InetSocketAddress
import java.net.Socket
import java.nio.charset.StandardCharsets
import java.security.SecureRandom
import java.util.concurrent.CountDownLatch
import java.util.concurrent.TimeUnit
import java.util.concurrent.atomic.AtomicReference

/**
 * Without `pairingBroker`, verifies ACTION_VIEW/appUrlOpen using a dummy URI.
 * With that non-secret host:port argument, fetches the valid URI in memory,
 * delivers ACTION_VIEW directly through BridgeActivity.onNewIntent (avoiding
 * Android's intent logger), pairs with the LAN harness, and scans logcat.
 */
@RunWith(AndroidJUnit4::class)
class DeepLinkDispatchTest {
    @get:Rule
    val grantCameraPermission = GrantPermissionRule.grant(Manifest.permission.CAMERA)

    @Test
    fun actionView_deliversAndRedactsPairingData() {
        val instrumentation = InstrumentationRegistry.getInstrumentation()
        val context = instrumentation.targetContext
        val args = InstrumentationRegistry.getArguments()
        val broker = args.getString(ARG_PAIRING_BROKER)
        val keepPairing = args.getString(ARG_KEEP_PAIRING) == "true"
        val holdStreamingMs = args.getString(ARG_HOLD_STREAMING_MS)?.toLongOrNull() ?: 0L
        val expectNotApproved = args.getString(ARG_EXPECT_NOT_APPROVED) == "true"
        val observeErrorBack = args.getString(ARG_OBSERVE_ERROR_BACK) == "true"
        val observeStopDialog = args.getString(ARG_OBSERVE_STOP_DIALOG) == "true"
        val waitForErrorBackMs = args.getString(ARG_WAIT_FOR_ERROR_BACK_MS)?.toLongOrNull() ?: 0L
        val waitForStopBackMs = args.getString(ARG_WAIT_FOR_STOP_BACK_MS)?.toLongOrNull() ?: 0L
        assertTrue("invalid requested stream hold interval", holdStreamingMs in 0..MAX_STREAM_HOLD_MS)
        assertTrue("invalid Error Back observation window", waitForErrorBackMs in 0..MAX_ERROR_BACK_WAIT_MS)
        assertTrue("invalid streaming Back observation window", waitForStopBackMs in 0..MAX_ERROR_BACK_WAIT_MS)
        assertTrue("Error Back observation requires an external Back window", !observeErrorBack || waitForErrorBackMs > 0)
        assertTrue("streaming Back observation requires an external Back window", !observeStopDialog || waitForStopBackMs > 0)
        assertTrue(
            "Error observation requires the real LAN broker",
            !(expectNotApproved || observeErrorBack || observeStopDialog) || broker != null,
        )
        val activity = instrumentation.startActivitySync(
            Intent(Intent.ACTION_MAIN)
                .setClassName(context.packageName, MainActivity::class.java.name)
                .addFlags(Intent.FLAG_ACTIVITY_NEW_TASK),
        ) as MainActivity
        val appPluginReady = awaitJavascript(activity) {
            "typeof window.Capacitor?.Plugins?.App?.addListener === 'function' && " +
                "typeof window.Capacitor?.Plugins?.LocalCamera?.addListener === 'function'"
        }
        assertTrue("Capacitor App listener was unavailable in the loaded WebView", appPluginReady == "true")
        assertTrue(
            "permissions screen could not advance to Home before the deep-link flow",
            advancePastPermissionsIfNeeded(activity),
        )

        val isRealPairing = broker != null
        val probe = if (isRealPairing) null else ByteArray(PROBE_BYTES).also(SecureRandom()::nextBytes).toHex()
        val uriText = broker?.let(::fetchPairingUri) ?: Uri.Builder()
            .scheme("localcamera")
            .authority("connect")
            .appendQueryParameter("probe", probe!!)
            .build()
            .toString()
        val uri = Uri.parse(uriText)
        val token = uri.getQueryParameter("token")
        val pcId = uri.getQueryParameter("pc")
        if (isRealPairing) {
            assertTrue("broker did not provide a valid pairing URI", uri.scheme == "localcamera" && token != null && pcId != null)
        }
        val installListener = """
            (() => {
              window.__lcDeepLinkProbeDelivered = false;
              window.__lcDeepLinkProbeReady = false;
              window.__lcConnectionState = '';
              window.__lcErrorCode = '';
              window.__lcBackButtonProbeDelivered = false;
              const app = window.Capacitor.Plugins.App;
              const local = window.Capacitor.Plugins.LocalCamera;
              const appRegistration = app.addListener('appUrlOpen', event => {
                window.__lcDeepLinkProbeDelivered = typeof event?.url === 'string' &&
                  event.url.startsWith('localcamera://connect');
              });
              const backRegistration = app.addListener('backButton', () => {
                window.__lcBackButtonProbeDelivered = true;
              });
              const connectionRegistration = local.addListener('connectionChanged', event => {
                window.__lcConnectionState = event?.state || '';
              });
              const errorRegistration = local.addListener('error', event => {
                window.__lcErrorCode = event?.code || '';
              });
              Promise.all([appRegistration, backRegistration, connectionRegistration, errorRegistration]).then(handles => {
                window.__lcDeepLinkProbeHandles = handles;
                window.__lcDeepLinkProbeReady = true;
              });
              return true;
            })()
        """.trimIndent()
        val listenerInstalled = evaluateJavascript(activity, installListener)
        var deviceKey: ByteArray? = null
        var foregroundMarked = false
        try {
            // As in ServiceSmokeTest, hold the same visibility guard open for
            // the instrumented app window while the valid link starts streaming.
            AppForeground.onActivityResumed()
            foregroundMarked = true
            assertTrue("temporary appUrlOpen observer was not installed", listenerInstalled == "true")
            assertTrue(
                "temporary appUrlOpen observer did not become ready",
                awaitJavascript(activity) { "window.__lcDeepLinkProbeReady === true" } == "true",
            )
            if (observeStopDialog) {
                assertTrue(
                    "could not disable background streaming through the visible Settings control",
                    disableBackgroundStreamingThroughUi(activity),
                )
            }
            Thread.sleep(LISTENER_SETTLE_MS)

            val viewIntent = Intent(Intent.ACTION_VIEW, uri)
                .setClassName(context.packageName, MainActivity::class.java.name)
            if (isRealPairing) dispatchInProcess(activity, viewIntent) else context.startActivity(
                viewIntent.addFlags(Intent.FLAG_ACTIVITY_NEW_TASK or Intent.FLAG_ACTIVITY_SINGLE_TOP),
            )

            val appUrlOpenDelivered = awaitJavascript(activity) {
                "window.__lcDeepLinkProbeDelivered === true"
            }
            if (isRealPairing && appUrlOpenDelivered == "true") {
                if (expectNotApproved) {
                    val deniedErrorDelivered = awaitJavascript(activity) {
                        "window.__lcErrorCode === 'denied_not_approved'"
                    }
                    val denialDiagnostic = evaluateJavascript(
                        activity,
                        "'state=' + (window.__lcConnectionState || 'none') + ';code=' + (window.__lcErrorCode || 'none')",
                    )?.trim('"')?.takeIf { SAFE_DIAGNOSTIC.matches(it) } ?: "unavailable"
                    assertTrue(
                        "server rejection did not reach the app as denied_not_approved ($denialDiagnostic)",
                        deniedErrorDelivered == "true",
                    )
                    val errorScreenVisible = awaitJavascript(activity) {
                        "Array.from(document.querySelectorAll('main h1')).some(h => " +
                            "['Conexão não aprovada', 'Connection not approved'].includes(h.textContent?.trim() || ''))"
                    }
                    assertTrue("not-approved event did not render the real Error screen", errorScreenVisible == "true")
                    if (waitForErrorBackMs > 0) {
                        Log.i(TAG, "AT212_ERROR_SCREEN_READY")
                        val returnedHome = awaitJavascript(activity, waitForErrorBackMs) {
                            "Array.from(document.querySelectorAll('main h2')).some(h => " +
                                "['Pronto para conectar', 'Ready to connect'].includes(h.textContent?.trim() || ''))"
                        }
                        assertTrue("Back from the real Error screen did not return to Home", returnedHome == "true")
                    }
                    val unexpectedKey = PairingStore(context).deviceKey(pcId!!)
                    unexpectedKey?.fill(0)
                    assertTrue("a denied pairing unexpectedly persisted a device key", unexpectedKey == null)
                } else if (observeErrorBack) {
                    val anyErrorDelivered = awaitJavascript(activity) {
                        "typeof window.__lcErrorCode === 'string' && window.__lcErrorCode.length > 0"
                    }
                    val errorCode = evaluateJavascript(activity, "window.__lcErrorCode || 'none'")
                        ?.trim('"')?.takeIf { SAFE_DIAGNOSTIC.matches(it) } ?: "unavailable"
                    assertTrue("no real app error event arrived for Error Back observation ($errorCode)", anyErrorDelivered == "true")
                    val errorScreenVisible = awaitJavascript(activity) {
                        "(() => { const main = document.querySelector('main'); return !!main?.querySelector('h1') && " +
                            "!!main.querySelector('p') && main.querySelectorAll('button').length >= 2; })()"
                    }
                    assertTrue("real app error event did not render the Error screen ($errorCode)", errorScreenVisible == "true")
                    Log.i(TAG, "AT212_ERROR_SCREEN_READY:$errorCode")
                    val homeHeading =
                        "Array.from(document.querySelectorAll('main h2')).some(h => " +
                            "['Pronto para conectar', 'Ready to connect'].includes(h.textContent?.trim() || ''))"
                    val returnedHome = awaitJavascript(activity, waitForErrorBackMs) {
                        homeHeading
                    }
                    val activityAvailable = !activity.isFinishing && !activity.isDestroyed
                    val backButtonDelivered = if (activityAvailable) {
                        evaluateJavascript(activity, "window.__lcBackButtonProbeDelivered === true")
                    } else {
                        null
                    }
                    Log.i(
                        TAG,
                        "AT212_BACK_RESULT:back=$backButtonDelivered;" +
                            "home=$returnedHome;finishing=${activity.isFinishing};destroyed=${activity.isDestroyed}",
                    )
                    assertTrue(
                        "ADB Back from the real Error screen did not return Home ($errorCode; " +
                            "back=$backButtonDelivered;home=$returnedHome;finishing=${activity.isFinishing})",
                        backButtonDelivered == "true" && returnedHome == "true",
                    )
                    val unexpectedKey = PairingStore(context).deviceKey(pcId!!)
                    unexpectedKey?.fill(0)
                    assertTrue("Error Back test unexpectedly persisted a device key", unexpectedKey == null)
                } else if (observeStopDialog) {
                    val streamingScreenVisible = awaitJavascript(activity) {
                        "window.__lcConnectionState === 'streaming' && " +
                            "Array.from(document.querySelectorAll('main button')).some(button => " +
                            "['Parar', 'Stop'].includes(button.textContent?.trim() || ''))"
                    }
                    val streamDiagnostic = evaluateJavascript(
                        activity,
                        "'state=' + (window.__lcConnectionState || 'none') + ';code=' + (window.__lcErrorCode || 'none')",
                    )?.trim('"')?.takeIf { SAFE_DIAGNOSTIC.matches(it) } ?: "unavailable"
                    assertTrue(
                        "real streaming screen did not appear before Back ($streamDiagnostic)",
                        streamingScreenVisible == "true",
                    )
                    Log.i(TAG, "AT212_STREAM_SCREEN_READY")
                    // The shadcn <Dialog> textContent concatenates the title with the
                    // Cancel/Stop buttons (no line break), so match by "contains title",
                    // not exact first line.
                    val stopDialogVisible = awaitJavascript(activity, waitForStopBackMs) {
                        "Array.from(document.querySelectorAll('[role=dialog]')).some(dialog => " +
                            "{ const tc = (dialog.textContent || '').replace(/\\s+/g, ' '); " +
                            "return ['Parar a transmissão?', 'Stop streaming?'].some(s => tc.includes(s)); })"
                    }
                    val backButtonDelivered = evaluateJavascript(
                        activity,
                        "window.__lcBackButtonProbeDelivered === true",
                    )
                    assertTrue(
                        "Back with background streaming disabled did not show the stop dialog " +
                            "(dialog=$stopDialogVisible;back=$backButtonDelivered)",
                        stopDialogVisible == "true" && backButtonDelivered == "true",
                    )
                } else {
                    deviceKey = awaitDeviceKey(context, pcId!!)
                    if (deviceKey != null && holdStreamingMs > 0) Thread.sleep(holdStreamingMs)
                }
            }
            val logScan = scanLogcat(instrumentation, secretForms(uriText, token, deviceKey, probe))
            val diagnostic = evaluateJavascript(
                activity,
                "'state=' + (window.__lcConnectionState || 'none') + ';code=' + (window.__lcErrorCode || 'none')",
            )?.trim('"')?.takeIf { SAFE_DIAGNOSTIC.matches(it) } ?: "state=unknown;code=unknown"
            if (args.getString(ARG_EXPECT_RECONNECT) == "true") {
                assertTrue(
                    "server shutdown did not surface reconnecting ($diagnostic)",
                    diagnostic.startsWith("state=reconnecting;"),
                )
            }
            assertTrue("ACTION_VIEW was not observed by the JS appUrlOpen listener", appUrlOpenDelivered == "true")
            // `deviceKey` is only awaited in the default pairing branch; the
            // observe* branches do not await it, so the persistence assertion
            // applies only there (it is separately covered by the normal pairing run).
            if (isRealPairing && !expectNotApproved && !observeErrorBack && !observeStopDialog) {
                assertTrue("pairing WELCOME did not persist the test device key ($diagnostic)", deviceKey != null)
            }
            assertTrue("pairing URI, token, device key, or their encoded form appeared in logcat", !logScan.secretObserved)
        } finally {
            if (isRealPairing) {
                StreamingService.stopStreaming(context)
                if (!keepPairing && pcId != null) PairingStore(context).forgetPc(pcId)
            }
            if (foregroundMarked) AppForeground.onActivityPaused()
            deviceKey?.fill(0)
            evaluateJavascript(
                activity,
                """
                    (() => {
                      window.__lcDeepLinkProbeHandles?.forEach(handle => handle?.remove());
                      delete window.__lcDeepLinkProbeHandles;
                      delete window.__lcDeepLinkProbeReady;
                      delete window.__lcDeepLinkProbeDelivered;
                      delete window.__lcConnectionState;
                      delete window.__lcErrorCode;
                      delete window.__lcBackButtonProbeDelivered;
                      return true;
                    })()
                """.trimIndent(),
            )
        }
    }

    private fun awaitJavascript(
        activity: MainActivity,
        timeoutMs: Long = WAIT_FOR_HANDLER_MS,
        expression: () -> String,
    ): String? {
        val deadline = System.currentTimeMillis() + timeoutMs
        while (System.currentTimeMillis() < deadline) {
            val result = evaluateJavascript(activity, expression())
            if (result == "true") return result
            Thread.sleep(POLL_INTERVAL_MS)
        }
        return null
    }

    private fun evaluateJavascript(activity: MainActivity, script: String): String? {
        val instrumentation = InstrumentationRegistry.getInstrumentation()
        val result = AtomicReference<String?>()
        val latch = CountDownLatch(1)
        instrumentation.runOnMainSync {
            activity.bridge.webView.evaluateJavascript(script) { value ->
                result.set(value)
                latch.countDown()
            }
        }
        if (!latch.await(JS_EVALUATION_TIMEOUT_MS, TimeUnit.MILLISECONDS)) return null
        return result.get()
    }

    /** Completes the real first-run permissions screen so it cannot mask later routes. */
    private fun advancePastPermissionsIfNeeded(activity: MainActivity): Boolean {
        val homeHeading =
            "Array.from(document.querySelectorAll('main h2')).some(h => " +
                "['Pronto para conectar', 'Ready to connect'].includes(h.textContent?.trim() || ''))"
        val uiReady = awaitJavascript(activity) {
            homeHeading + " || Array.from(document.querySelectorAll('main button')).some(button => " +
                "['Continuar', 'Continue'].includes(button.textContent?.trim() || ''))"
        }
        if (uiReady != "true") return false
        if (evaluateJavascript(activity, homeHeading) == "true") return true

        val result = evaluateJavascript(
            activity,
            """
                (() => {
                  const button = Array.from(document.querySelectorAll('main button')).find(element =>
                    ['Continuar', 'Continue'].includes(element.textContent?.trim() || '')
                  );
                  if (!button) return 'already-advanced';
                  if (button.disabled) return 'blocked';
                  button.click();
                  return 'clicked';
                })()
            """.trimIndent(),
        )?.trim('"')
        if (result != "clicked") return false
        return awaitJavascript(activity) { homeHeading } == "true"
    }

    /** Uses the visible Settings controls; never dispatches or mutates AppState directly. */
    private fun disableBackgroundStreamingThroughUi(activity: MainActivity): Boolean {
        val opened = evaluateJavascript(
            activity,
            """
                (() => {
                  const button = Array.from(document.querySelectorAll('button')).find(element =>
                    ['Configurações', 'Settings'].includes(element.getAttribute('aria-label') || '')
                  );
                  if (!button) return 'missing-settings';
                  button.click();
                  return 'opened';
                })()
            """.trimIndent(),
        )?.trim('"')
        if (opened != "opened") return false

        val toggleSelector =
            "Array.from(document.querySelectorAll('[role=dialog] button')).find(button => " +
                "['Continuar transmitindo com a tela apagada', 'Continue streaming with the screen off']" +
                ".includes(button.textContent?.trim() || ''))"
        if (awaitJavascript(activity) { "$toggleSelector !== undefined && !!($toggleSelector)" } != "true") {
            return false
        }
        val initialPressed = evaluateJavascript(activity, "($toggleSelector).getAttribute('aria-pressed')")
            ?.trim('"')
        if (initialPressed == "true") evaluateJavascript(activity, "($toggleSelector).click(); true")
        if (awaitJavascript(activity) { "($toggleSelector).getAttribute('aria-pressed') === 'false'" } != "true") {
            return false
        }

        val closeClicked = evaluateJavascript(
            activity,
            """
                (() => {
                  const dialog = document.querySelector('[role=dialog]');
                  const button = Array.from(dialog?.querySelectorAll('button') || []).find(element =>
                    ['Fechar', 'Close'].includes(element.getAttribute('aria-label') || '')
                  );
                  if (!button) return false;
                  button.click();
                  return true;
                })()
            """.trimIndent(),
        )
        if (closeClicked != "true") return false
        return awaitJavascript(activity) {
            "!document.querySelector('[role=dialog]') && " +
                "Array.from(document.querySelectorAll('main h2')).some(h => " +
                "['Pronto para conectar', 'Ready to connect'].includes(h.textContent?.trim() || ''))"
        } == "true"
    }

    /** Calls the same Capacitor lifecycle route without ActivityTaskManager logging URI data. */
    private fun dispatchInProcess(activity: MainActivity, intent: Intent) {
        val method = BridgeActivity::class.java.getDeclaredMethod("onNewIntent", Intent::class.java)
        method.isAccessible = true
        InstrumentationRegistry.getInstrumentation().runOnMainSync {
            method.invoke(activity, intent)
        }
    }

    /** The broker address is non-secret. The URI body is kept in memory only. */
    private fun fetchPairingUri(address: String): String {
        val endpoint = Uri.parse("http://$address/pairing")
        assertTrue("invalid in-memory pairing broker address", endpoint.host != null && endpoint.port in 1..65_535)
        val host = endpoint.host!!
        val response = try {
            Socket().use { socket ->
                socket.connect(InetSocketAddress(host, endpoint.port), BROKER_CONNECT_TIMEOUT_MS)
                socket.soTimeout = BROKER_READ_TIMEOUT_MS
                socket.getOutputStream().write(
                    "GET /pairing HTTP/1.1\r\nHost: $host\r\nConnection: close\r\n\r\n"
                        .toByteArray(StandardCharsets.US_ASCII),
                )
                socket.getInputStream().bufferedReader(StandardCharsets.UTF_8).readText()
            }
        } catch (_: Exception) {
            throw AssertionError("could not read from in-memory pairing broker")
        }
        val body = response.substringAfter("\r\n\r\n", "")
        assertTrue("in-memory pairing broker returned no pairing URI", response.startsWith("HTTP/1.1 200 ") && body.startsWith("localcamera://connect?"))
        return body
    }

    private fun awaitDeviceKey(context: android.content.Context, pcId: String): ByteArray? {
        val store = PairingStore(context)
        val deadline = System.currentTimeMillis() + PAIRING_TIMEOUT_MS
        while (System.currentTimeMillis() < deadline) {
            store.deviceKey(pcId)?.let { return it }
            Thread.sleep(POLL_INTERVAL_MS)
        }
        return null
    }

    /** Log lines are inspected in memory and discarded; raw output is never retained. */
    private fun scanLogcat(
        instrumentation: android.app.Instrumentation,
        secretForms: List<String>,
    ): ScanResult {
        val descriptor = instrumentation.uiAutomation.executeShellCommand("logcat -d -v brief")
        var secretObserved = false
        ParcelFileDescriptor.AutoCloseInputStream(descriptor)
            .bufferedReader(StandardCharsets.UTF_8)
            .useLines { lines ->
                lines.forEach { line ->
                    if (!secretObserved && secretForms.any(line::contains)) secretObserved = true
                }
            }
        return ScanResult(secretObserved)
    }

    private fun secretForms(uri: String, token: String?, deviceKey: ByteArray?, probe: String?): List<String> {
        val uriBytes = uri.toByteArray(StandardCharsets.UTF_8)
        val forms = mutableListOf(
            Uri.encode(uri),
            Base64.encodeToString(uriBytes, Base64.NO_WRAP),
            Base64.encodeToString(uriBytes, Base64.URL_SAFE or Base64.NO_WRAP or Base64.NO_PADDING),
            uri,
        )
        token?.let { value ->
            forms += value
            val bytes = value.toByteArray(StandardCharsets.UTF_8)
            forms += Base64.encodeToString(bytes, Base64.NO_WRAP)
            forms += Base64.encodeToString(bytes, Base64.URL_SAFE or Base64.NO_WRAP or Base64.NO_PADDING)
            value.hexBytesOrNull()?.let { tokenBytes ->
                forms += Base64.encodeToString(tokenBytes, Base64.NO_WRAP)
                forms += Base64.encodeToString(tokenBytes, Base64.URL_SAFE or Base64.NO_WRAP or Base64.NO_PADDING)
            }
        }
        deviceKey?.let { bytes ->
            val hex = bytes.toHex()
            forms += hex
            forms += hex.uppercase()
            forms += Base64.encodeToString(bytes, Base64.NO_WRAP)
            forms += Base64.encodeToString(bytes, Base64.URL_SAFE or Base64.NO_WRAP or Base64.NO_PADDING)
            forms += Base64.encodeToString(hex.toByteArray(StandardCharsets.US_ASCII), Base64.NO_WRAP)
        }
        probe?.let(forms::add)
        return forms.filter(String::isNotEmpty)
    }

    private fun ByteArray.toHex(): String = joinToString("") { "%02x".format(it) }

    private fun String.hexBytesOrNull(): ByteArray? {
        if (length % 2 != 0 || any { it.digitToIntOrNull(16) == null }) return null
        return chunked(2).map { it.toInt(16).toByte() }.toByteArray()
    }

    private data class ScanResult(val secretObserved: Boolean)

    private companion object {
        const val PROBE_BYTES = 16
        const val ARG_PAIRING_BROKER = "pairingBroker"
        const val ARG_KEEP_PAIRING = "keepPairing"
        const val ARG_HOLD_STREAMING_MS = "holdStreamingMs"
        const val ARG_EXPECT_RECONNECT = "expectReconnect"
        const val ARG_EXPECT_NOT_APPROVED = "expectNotApproved"
        const val ARG_OBSERVE_ERROR_BACK = "observeErrorBack"
        const val ARG_WAIT_FOR_ERROR_BACK_MS = "waitForErrorBackMs"
        const val ARG_OBSERVE_STOP_DIALOG = "observeStopDialog"
        const val ARG_WAIT_FOR_STOP_BACK_MS = "waitForStopBackMs"
        const val WAIT_FOR_HANDLER_MS = 12_000L
        const val MAX_ERROR_BACK_WAIT_MS = 30_000L
        const val PAIRING_TIMEOUT_MS = 45_000L
        // Allows LT-2.01's single real stream soak; caller supplies duration explicitly.
        const val MAX_STREAM_HOLD_MS = 1_860_000L
        const val POLL_INTERVAL_MS = 400L
        const val LISTENER_SETTLE_MS = 750L
        const val JS_EVALUATION_TIMEOUT_MS = 2_000L
        const val BROKER_CONNECT_TIMEOUT_MS = 5_000
        const val BROKER_READ_TIMEOUT_MS = 5_000
        const val TAG = "DeepLinkDispatchTest"
        val SAFE_DIAGNOSTIC = Regex("[a-zA-Z0-9_=;.-]{1,80}")
    }
}
