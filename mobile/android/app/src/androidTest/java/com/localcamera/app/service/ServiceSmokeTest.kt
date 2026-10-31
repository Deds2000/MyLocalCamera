package com.localcamera.app.service

import android.Manifest
import android.app.Notification
import android.app.NotificationManager
import android.content.Context
import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.platform.app.InstrumentationRegistry
import androidx.test.rule.GrantPermissionRule
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertNull
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith

/**
 * Smoke (cartão F2-T12, passo 5): start/stop do serviço com alvo **fake**
 * (`192.0.2.1` = TEST-NET, nunca conecta) → notificação criada no canal
 * `streaming` com a ação "Parar" e removida no stop (via `NotificationManager`).
 *
 * Requer aparelho (IT-2.07 valida o texto em pt-BR/en manualmente). A permissão
 * `POST_NOTIFICATIONS` precisa estar concedida no aparelho (Android 13+),
 * senão a notificação não é exibida nem consultável.
 */
@RunWith(AndroidJUnit4::class)
class ServiceSmokeTest {

    @get:Rule
    val grantCamera = GrantPermissionRule.grant(Manifest.permission.CAMERA)

    // Android 13+: sem ela a notificação do FGS não aparece (ver comentário acima).
    @get:Rule
    val grantNotifications = GrantPermissionRule.grant(Manifest.permission.POST_NOTIFICATIONS)

    @Test
    fun start_creates_notification_and_stop_removes_it() {
        val context = InstrumentationRegistry.getInstrumentation().targetContext.applicationContext
        val nm = context.getSystemService(Context.NOTIFICATION_SERVICE) as NotificationManager

        // O teste roda sem activity: simulamos app visível para o smoke não
        // ser bloqueado pela guarda da armadilha §6 (que é coberta no GATE).
        AppForeground.onActivityResumed()
        try {
            StreamingService.stopStreaming(context)
            Thread.sleep(SETTLE_MS)
            assertNull("estado inicial", currentNotif(nm))

            StreamingService.startStreaming(context, FAKE_URI)
            val created = poll(nm, UP_TO_MS) { it != null }
            assertNotNull("notificação não criada após o start", created)
            val actions = created!!.actions
            assertNotNull("notificação sem ações", actions)
            assertEquals("ação 'Parar' ausente", 1, actions.size)

            StreamingService.stopStreaming(context)
            val removed = poll(nm, UP_TO_MS) { it == null }
            assertNull("notificação não removida após o stop", removed)
        } finally {
            AppForeground.onActivityPaused()
        }
    }

    /**
     * Notificação da sessão entre as ativas, por `id` (a 2-arg
     * `getNotification(channelId,id)` saiu do android.jar 36).
     */
    private fun currentNotif(nm: NotificationManager): Notification? =
        nm.activeNotifications.firstOrNull { it.id == StreamingService.NOTIF_ID }?.notification

    /** Consulta a notificação até [upToMs] ou o fim do prazo; devolve o último visto. */
    private fun poll(nm: NotificationManager, upToMs: Long, cond: (Notification?) -> Boolean): Notification? {
        val deadline = System.currentTimeMillis() + upToMs
        var last: Notification? = null
        while (System.currentTimeMillis() < deadline) {
            last = currentNotif(nm)
            if (cond(last)) return last
            Thread.sleep(POLL_MS)
        }
        return last
    }

    private companion object {
        /** URI válida com host de TEST-NET (RFC 5737): o handshake nunca completa. */
        const val FAKE_URI =
            "localcamera://connect?v=1&hosts=192.0.2.1&port=48650" +
                "&token=0123456789abcdef0123456789abcdef" +
                "&pc=123e4567-e89b-42d3-a456-426614174000&name=FakePC"

        const val POLL_MS = 250L
        const val SETTLE_MS = 500L
        const val UP_TO_MS = 30_000L
    }
}
