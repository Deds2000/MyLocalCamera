package com.localcamera.app.camera

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

/** Seleção do modo de sessão (decisão 2026-11-02): melhor 16:9 ≤ 1080p por probe. */
class SessionModePickerTest {

    /** Lista no formato do probe do Samsung (1080p + 4K). */
    private val samsungLike = listOf(
        SessionMode(1920, 1080, 30),
        SessionMode(1280, 720, 30),
        SessionMode(3840, 2160, 30), // 4K: fora do teto
    )

    /** Lista no formato do probe do Moto G (5): 16:9 até 720p + 4:3. */
    private val motoLike = listOf(
        SessionMode(1280, 720, 30),
        SessionMode(1280, 960, 30), // 4:3: fora
        SessionMode(960, 720, 30), // 4:3: fora
        SessionMode(720, 480, 30),
        SessionMode(768, 432, 30),
        SessionMode(640, 480, 30), // 4:3: fora
    )

    @Test
    fun escolhe_1080p_no_samsung() {
        assertEquals(SessionMode(1920, 1080, 30), SessionModePicker.pick(samsungLike))
    }

    @Test
    fun escolhe_720p_no_moto_sem_1080p() {
        assertEquals(SessionMode(1280, 720, 30), SessionModePicker.pick(motoLike))
    }

    @Test
    fun lista_vazia_da_null() {
        assertNull(SessionModePicker.pick(emptyList()))
    }

    @Test
    fun so_4_3_nao_elegivel_da_null() {
        assertNull(SessionModePicker.pick(listOf(SessionMode(1280, 960, 30))))
    }

    @Test
    fun teto_menor_que_todos_da_null() {
        assertNull(
            SessionModePicker.pick(
                listOf(SessionMode(1280, 720, 30)),
                ceiling = SessionMode(640, 360, 30),
            ),
        )
    }

    @Test
    fun fps_invalido_e_dimensoes_nulas_sao_ignorados() {
        assertNull(
            SessionModePicker.pick(
                listOf(SessionMode(1920, 1080, 0), SessionMode(0, 0, 30)),
            ),
        )
    }

    @Test
    fun empate_de_area_mantem_o_primeiro_da_lista() {
        val modos = listOf(SessionMode(1280, 720, 30), SessionMode(1280, 720, 30))
        assertEquals(modos.first(), SessionModePicker.pick(modos))
    }

    @Test
    fun is16x9_reconhece_somente_o_aspecto_exato() {
        assertTrue(SessionModePicker.is16x9(1920, 1080))
        assertTrue(SessionModePicker.is16x9(1280, 720))
        assertTrue(SessionModePicker.is16x9(320, 180))
        assertFalse(SessionModePicker.is16x9(1280, 960))
        assertFalse(SessionModePicker.is16x9(1920, 1081))
        assertFalse(SessionModePicker.is16x9(0, 1080))
    }

    @Test
    fun default_e_o_teto_do_cartao() {
        assertEquals(SessionMode(1920, 1080, 30), SessionModePicker.DEFAULT)
    }
}
