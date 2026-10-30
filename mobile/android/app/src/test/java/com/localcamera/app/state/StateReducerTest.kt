package com.localcamera.app.state

import com.localcamera.app.protocol.AudioState
import com.localcamera.app.protocol.CameraState
import com.localcamera.app.protocol.Presentation
import com.localcamera.app.protocol.Sides
import com.localcamera.app.protocol.VideoState
import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test
import java.io.File

/**
 * UT-2.06 — `StateReducer`: estado mínimo da Fase 2, `rev` incrementa a cada
 * mudança, `STATE` no formato exato do vetor `state_basic` e aplicação de
 * `SET` (vetores `set_single`/`set_list` de `docs/protocol-vectors.json`).
 */
class StateReducerTest {

    companion object {
        /** Formato canônico compacto do STATE (mínimo da Fase 2; vetor `state_basic`). */
        private const val EXACT_STATE_BASIC =
            """{"rev":1,"camera":{"mode":"single","active":"back"},"video":{"width":1920,"height":1080,"fps":30,"bitrateKbps":8000},"presentation":{"mirror":{"back":false,"front":false},"rotation":{"back":0,"front":0}},"audio":{"enabled":true,"gain":1.0}}"""

        private val vectorsFile = File("../../../docs/protocol-vectors.json")
        private val vectorsText: String = run {
            assertTrue("arquivo de vetores não encontrado: rode a partir da raiz do repo", vectorsFile.exists())
            vectorsFile.readText(Charsets.UTF_8)
        }

        /**
         * Texto cru do campo `json` do vetor (objeto `{}` ou lista `[]`),
         * extraído do arquivo — o `JSONObject` do org.json não preserva ordem
         * de campos nem a forma `1.0` de um double inteiro, e o byte a byte
         * do `STATE` importa aqui.
         */
        private fun rawJson(name: String): String {
            val anchor = vectorsText.indexOf("\"name\": \"$name\"")
            check(anchor >= 0) { "vetor não encontrado: $name" }
            val colon = vectorsText.indexOf("\"json\":", anchor)
            check(colon >= 0) { "vetor $name sem campo 'json'" }
            val rest = vectorsText.substring(colon)
            val start = rest.indexOfFirst { it == '{' || it == '[' }
            check(start >= 0) { "vetor $name: 'json' não é objeto nem lista" }
            val open = rest[start]
            val close = if (open == '{') '}' else ']'
            var depth = 0
            for (i in start until rest.length) {
                when (rest[i]) {
                    open -> depth++
                    close -> {
                        depth--
                        if (depth == 0) return vectorsText.substring(colon + start, colon + i + 1)
                    }
                }
            }
            error("vetor $name: objeto incompleto")
        }
    }

    // UT-2.06: estado inicial = mínimo da Fase 2 (padrão do vetor state_basic).
    @Test
    fun ut206_initialState() {
        val r = StateReducer()
        assertEquals(1L, r.state.rev)
        assertEquals(CameraState("single", "back"), r.state.camera)
        assertEquals(VideoState(1920, 1080, 30, 8000), r.state.video)
        assertEquals(Presentation(Sides(false, false), Sides(0, 0)), r.state.presentation)
        assertEquals(AudioState(true, 1.0), r.state.audio)
    }

    // UT-2.06: stateJson() no formato exato do vetor state_basic.
    @Test
    fun ut206_stateJsonExactVectorFormat() {
        // O vetor no arquivo vem formatado (espaços); o formato canônico do
        // STATE é compacto — confere byte a byte e, semântica, contra o vetor
        // (via toString normalizado: o equals do org.json 20260814 é bugado).
        assertEquals(EXACT_STATE_BASIC, StateReducer().stateJson())
        assertEquals(
            JSONObject(EXACT_STATE_BASIC).toString(),
            JSONObject(rawJson("state_basic")).toString(),
        )
    }

    // UT-2.06: set_single aplica camera.active, incrementa rev e reflete no STATE.
    @Test
    fun ut206_setSingleIncrementsRev() {
        val r = StateReducer()
        val after = r.applySet(rawJson("set_single")) // {"key":"camera.active","value":"front"}
        assertEquals("front", after.camera.active)
        assertEquals(2L, after.rev)
        assertEquals(after, r.state)

        val o = JSONObject(r.stateJson())
        assertEquals(2, o.getLong("rev"))
        assertEquals("front", o.getJSONObject("camera").getString("active"))
        assertEquals(1920, o.getJSONObject("video").getInt("width"))
        assertEquals(1.0, o.getJSONObject("audio").getDouble("gain"), 0.0)
    }

    // UT-2.06: set_list (fps sem mudança + width muda) → um único incremento.
    @Test
    fun ut206_setListSingleIncrement() {
        val r = StateReducer()
        r.applySet(rawJson("set_list")) // [{"video.fps":30},{"video.width":1280}]
        assertEquals(30, r.state.video.fps)
        assertEquals(1280, r.state.video.width)
        assertEquals(1080, r.state.video.height) // intocado
        assertEquals(2L, r.state.rev)
    }

    // UT-2.06: chaves fora da Fase 2, valores inválidos e JSON malformado
    // não mudam estado nem rev (e logam).
    @Test
    fun ut206_unknownAndInvalidIgnored() {
        val logs = mutableListOf<String>()
        val r = StateReducer { logs += it }
        r.applySet("""{"key":"audio.gain","value":1.5}""")      // Fase 8
        r.applySet("""{"key":"camera.mode","value":"dual"}""")  // Fase 7
        r.applySet("""{"key":"presentation.rotation.back","value":45}""") // fora de 0/90/180/270
        r.applySet("""{"key":"video.fps","value":"fast"}""")    // tipo errado
        r.applySet("isto não é json")
        assertEquals(AppState.initial(), r.state)
        assertEquals(1L, r.state.rev)
        assertTrue("deveria logar os SETs ignorados", logs.isNotEmpty())
    }

    // UT-2.06: SET com o mesmo valor → sem incremento de rev.
    @Test
    fun ut206_noOpSetKeepsRev() {
        val r = StateReducer()
        r.applySet("""{"key":"video.fps","value":30}""")
        r.applySet("""[{"key":"camera.active","value":"back"},{"key":"presentation.mirror.front","value":false}]""")
        assertEquals(1L, r.state.rev)
        assertEquals(EXACT_STATE_BASIC, r.stateJson())
    }

    // UT-2.06: rotação → onRotation só quando a rotação da câmera ativa muda.
    @Test
    fun ut206_rotationCallback() {
        val events = mutableListOf<Int>()
        val r = StateReducer()
        r.onRotation = { events += it }

        r.applySet("""{"key":"presentation.rotation.back","value":90}""") // ativa = back
        assertEquals(listOf(90), events)
        assertEquals(2L, r.state.rev)

        r.applySet("""{"key":"presentation.rotation.front","value":180}""") // ativa ainda back
        assertEquals(listOf(90), events)
        assertEquals(3L, r.state.rev)

        r.applySet(rawJson("set_single")) // ativa → front (rotação 180)
        assertEquals(listOf(90, 180), events)
        assertEquals(4L, r.state.rev)
        assertEquals("front", r.state.camera.active)
    }

    // UT-2.06: presentation.mirror.* e apresentação geral aplicam e refletem no STATE.
    @Test
    fun ut206_presentationKeys() {
        val r = StateReducer()
        r.applySet(
            """[{"key":"presentation.mirror.back","value":true},
               |{"key":"presentation.rotation.front","value":270}]""".trimMargin(),
        )
        assertTrue(r.state.presentation.mirror.back)
        assertFalse(r.state.presentation.mirror.front)
        assertEquals(0, r.state.presentation.rotation.back)
        assertEquals(270, r.state.presentation.rotation.front)
        assertEquals(2L, r.state.rev) // 1 chamada com mudanças → +1

        val p = JSONObject(r.stateJson()).getJSONObject("presentation")
        assertTrue(p.getJSONObject("mirror").getBoolean("back"))
        assertEquals(270, p.getJSONObject("rotation").getInt("front"))
    }
}
