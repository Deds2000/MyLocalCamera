package com.localcamera.app.camera

/**
 * Face da câmera — mesmo código do estado do protocolo
 * (`camera.active` = "back" | "front"; CHEATSHEET §Estado compartilhado).
 * Kotlin puro: nenhum tipo Android aqui (testável em JVM).
 */
enum class Facing(val code: String) {
    BACK("back"),
    FRONT("front");

    companion object {
        /** Decodifica o código do protocolo; `null` se desconhecido (ex.: "external"). */
        fun fromCode(code: String): Facing? = entries.firstOrNull { it.code == code }
    }
}

/**
 * Dados mínimos da câmera (cartão F2-T07): face + resolução + fps.
 * Padrão do cartão: 1080p30 (CHEATSHEET). 60 fps **nunca é oferecido**
 * (S3: `offers60=false`): a sonda (F2-T06) nunca o emite; aqui o fps é
 * validado apenas como `> 0` e a faixa AE `[fps, fps]` vai ao hardware —
 * aparelho sem a faixa cai no teto (S3 T3a: alvo 60 → rodou 30).
 */
data class CameraConfig(
    val facing: Facing,
    val width: Int = 1920,
    val height: Int = 1080,
    val fps: Int = 30,
) {
    init {
        require(width > 0 && height > 0) { "resolução ${width}x$height deve ser > 0" }
        require(fps > 0) { "fps $fps deve ser > 0" }
    }
}

/**
 * Hint de rotação (0/90/180/270): ângulo que o PC aplica ao stream para
 * exibi-lo direito (alimenta o `rotationHint` do STREAM_CONFIG — F2-T12,
 * AT-2.14).
 *
 * Fórmula medida no S3 (sensor traseiro 90° → retrato 90°, paisagem 0°;
 * fonte: `spikes/s3/SpikeTest.kt`):
 * - traseira: (sensor − rotaçãoDoAparelho) mod 360
 * - frontal:  (rotaçãoDoAparelho − sensor) mod 360 (espelhada; validada no
 *   aparelho com frente no AT-2.14/F2-T14)
 *
 * Puro (testável em JVM): o sensor vem de `CameraInfo.sensorRotationDegrees`
 * e a rotação do aparelho de `DisplayManager` (injetável em [CameraXBackend]).
 */
object RotationHint {

    fun forCamera(facing: Facing, sensorOrientationDeg: Int, deviceRotationDeg: Int): Int =
        when (facing) {
            Facing.BACK -> mod360(sensorOrientationDeg - deviceRotationDeg)
            Facing.FRONT -> mod360(deviceRotationDeg - sensorOrientationDeg)
        }

    private fun mod360(deg: Int): Int = ((deg % 360) + 360) % 360
}
