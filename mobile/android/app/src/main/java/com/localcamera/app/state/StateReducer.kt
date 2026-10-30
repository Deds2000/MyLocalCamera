package com.localcamera.app.state

import android.util.Log
import com.localcamera.app.protocol.AudioState
import com.localcamera.app.protocol.CameraState
import com.localcamera.app.protocol.CodecError
import com.localcamera.app.protocol.Presentation
import com.localcamera.app.protocol.SetItem
import com.localcamera.app.protocol.SetPayload
import com.localcamera.app.protocol.SetValue
import com.localcamera.app.protocol.Sides
import com.localcamera.app.protocol.VideoState
import com.localcamera.app.protocol.decodeSet
import org.json.JSONObject

/**
 * Estado compartilhado da Fase 2 — o celular é o dono (protocolo §9).
 * Reaproveita os tipos do modelo do F2-T04 (mesmos nomes de campo dos
 * vetores); aqui os sub-estados são sempre presentes (não nulos).
 * [rev] começa em 1 e incrementa a cada mudança aplicada.
 */
data class AppState(
    val rev: Long,
    val camera: CameraState,
    val video: VideoState,
    val presentation: Presentation,
    val audio: AudioState,
) {
    companion object {
        /** Estado mínimo da Fase 2 (exatamente o vetor `state_basic`). */
        fun initial(): AppState = AppState(
            rev = 1,
            camera = CameraState(mode = "single", active = "back"),
            video = VideoState(width = 1920, height = 1080, fps = 30, bitrateKbps = 8000),
            presentation = Presentation(
                mirror = Sides(back = false, front = false),
                rotation = Sides(back = 0, front = 0),
            ),
            audio = AudioState(enabled = true, gain = 1.0),
        )
    }
}

/**
 * Reducer único de mudanças de estado (protocolo §9): mudanças da UI do
 * celular e `SET`s do PC passam pelo mesmo caminho. Puro (sem I/O): aplica
 * as chaves da Fase 2 na ordem de chegada e devolve o novo estado; quem
 * chama (F2-T09) envia `STATE` com [stateJson] depois de cada mudança.
 *
 * Chaves da Fase 2: `camera.active`, `video.{width,height,fps,bitrateKbps}`,
 * `presentation.*`. `camera.mode` (dual, Fase 7) e `audio.*` (Fase 8) ainda
 * não se aplicam, assim como chaves desconhecidas: são ignoradas com log.
 */
class StateReducer(
    private val log: (String) -> Unit = { msg -> Log.d(TAG, msg) },
) {
    /** Estado atual; cada mudança devolve uma cópia (nunca muta no lugar). */
    var state: AppState = AppState.initial()
        private set

    /**
     * Mudança aplicada de `presentation.rotation` → nova rotação (0/90/180/270)
     * da câmera **ativa** (a que o render exibe) — alimenta `onRotationHint`
     * do F2-T07; o F2-T12 faz a ligação.
     */
    var onRotation: ((Int) -> Unit)? = null

    /**
     * Aplica um `SET` (PC→celular): `{key,value}` ou lista de pares
     * (decodificação via `Messages` do F2-T04). Incrementa [rev] no máximo
     * uma vez por chamada (uma `SET` = uma mudança = um `STATE`).
     * Chave desconhecida, valor inválido ou JSON malformado → estado igual,
     * sem exceção. Devolve o novo estado (mesma referência se não mudou).
     */
    @Synchronized
    fun applySet(json: String): AppState {
        val items: List<SetItem> = try {
            when (val p = decodeSet(json.toByteArray(Charsets.UTF_8))) {
                is SetPayload.One -> listOf(p.item)
                is SetPayload.Many -> p.items
            }
        } catch (e: CodecError) {
            log("SET inválido ignorado (${e.kind})")
            return state
        }
        if (items.isEmpty()) return state

        var next = state
        var changed = false
        for (item in items) {
            val (st, outcome) = applyOne(next, item.key, item.value)
            when (outcome) {
                Outcome.Applied -> { next = st; changed = true }
                Outcome.NoChange -> Unit
                Outcome.Invalid -> log("SET ignorado: valor inválido para '${item.key}'")
                Outcome.Unknown -> log("SET ignorado (fora da Fase 2): '${item.key}'")
            }
        }
        if (!changed) return state

        next = next.copy(rev = next.rev + 1)
        val oldRot = state.presentation.rotation.of(state.camera.active)
        val newRot = next.presentation.rotation.of(next.camera.active)
        if (newRot != oldRot) onRotation?.invoke(newRot)
        state = next
        return next
    }

    /**
     * `STATE` no formato exato do vetor `state_basic` (mesmos campos, mesma
     * ordem, `gain` sempre com decimal). Construído por mão porque o
     * `JSONObject` do org.json não preserva ordem de inserção.
     */
    fun stateJson(): String = buildString {
        append("{\"rev\":").append(state.rev)
        append(",\"camera\":{\"mode\":").append(JSONObject.quote(state.camera.mode))
        append(",\"active\":").append(JSONObject.quote(state.camera.active)).append('}')
        append(",\"video\":{\"width\":").append(state.video.width)
        append(",\"height\":").append(state.video.height)
        append(",\"fps\":").append(state.video.fps)
        append(",\"bitrateKbps\":").append(state.video.bitrateKbps).append('}')
        append(",\"presentation\":{\"mirror\":{\"back\":").append(state.presentation.mirror.back)
        append(",\"front\":").append(state.presentation.mirror.front)
        append("},\"rotation\":{\"back\":").append(state.presentation.rotation.back)
        append(",\"front\":").append(state.presentation.rotation.front).append("}}")
        append(",\"audio\":{\"enabled\":").append(state.audio.enabled)
        append(",\"gain\":").append(state.audio.gain.toString()).append("}")
        append('}')
    }

    // ------------------------------ aplicação ------------------------------

    private enum class Outcome { Applied, NoChange, Invalid, Unknown }

    private fun applyOne(cur: AppState, key: String, value: SetValue): Pair<AppState, Outcome> =
        when (key) {
            "camera.active" -> when (value) {
                is SetValue.Str ->
                    if (value.value == "back" || value.value == "front") {
                        if (value.value == cur.camera.active) cur to Outcome.NoChange
                        else cur.copy(camera = cur.camera.copy(active = value.value)) to Outcome.Applied
                    } else {
                        cur to Outcome.Invalid
                    }
                else -> cur to Outcome.Invalid
            }
            "video.width" -> videoNum(cur, value) { v, n -> v.copy(width = n) }
            "video.height" -> videoNum(cur, value) { v, n -> v.copy(height = n) }
            "video.fps" -> videoNum(cur, value) { v, n -> v.copy(fps = n) }
            "video.bitrateKbps" -> videoNum(cur, value) { v, n -> v.copy(bitrateKbps = n) }
            "presentation.mirror.back" -> mirror(cur, value, back = true)
            "presentation.mirror.front" -> mirror(cur, value, back = false)
            "presentation.rotation.back" -> rotation(cur, value, back = true)
            "presentation.rotation.front" -> rotation(cur, value, back = false)
            // Fora da Fase 2: camera.mode (dual, Fase 7) e audio.* (Fase 8).
            else -> cur to Outcome.Unknown
        }

    private fun videoNum(
        cur: AppState,
        value: SetValue,
        update: (VideoState, Int) -> VideoState,
    ): Pair<AppState, Outcome> {
        val n = (value as? SetValue.IntNum)?.value?.toInt() ?: return cur to Outcome.Invalid
        if (n <= 0) return cur to Outcome.Invalid
        val nv = update(cur.video, n)
        return if (nv == cur.video) cur to Outcome.NoChange
        else cur.copy(video = nv) to Outcome.Applied
    }

    private fun mirror(cur: AppState, value: SetValue, back: Boolean): Pair<AppState, Outcome> {
        val b = (value as? SetValue.Bool)?.value ?: return cur to Outcome.Invalid
        val m = cur.presentation.mirror
        if (b == (if (back) m.back else m.front)) return cur to Outcome.NoChange
        val nm = Sides(if (back) b else m.back, if (back) m.front else b)
        return cur.copy(presentation = cur.presentation.copy(mirror = nm)) to Outcome.Applied
    }

    private fun rotation(cur: AppState, value: SetValue, back: Boolean): Pair<AppState, Outcome> {
        val n = (value as? SetValue.IntNum)?.value?.toInt() ?: return cur to Outcome.Invalid
        if (n !in ROTATIONS) return cur to Outcome.Invalid
        val r = cur.presentation.rotation
        if (n == (if (back) r.back else r.front)) return cur to Outcome.NoChange
        val nr = Sides(if (back) n else r.back, if (back) r.front else n)
        return cur.copy(presentation = cur.presentation.copy(rotation = nr)) to Outcome.Applied
    }

    private fun Sides<Int>.of(active: String): Int = if (active == "front") front else back

    private companion object {
        private const val TAG = "StateReducer"
        private val ROTATIONS = setOf(0, 90, 180, 270)
    }
}
