package com.localcamera.app.uri

import org.json.JSONArray
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test
import java.io.File

/**
 * UT-2.03 — parser da URI de pareamento (F2-T05a).
 *
 * 16 vetores canônicos (`docs/pairing-uri-vectors.json`) + casos extras do
 * cartão: port 0/70000/ausente, hosts vazio/IPv6/6 itens, v=2, token curto/
 * ausente/maiúsculo, pc ausente/vazio/não-UUID, name ausente/vazio e
 * parâmetros extras desconhecidos.
 */
class PairingUriVectorsTest {

    companion object {
        private const val TOKEN = "0123456789abcdef0123456789abcdef"
        private const val PC = "11111111-1111-4111-8111-111111111111"
        private const val BASE =
            "localcamera://connect?v=1&hosts=192.168.0.10&port=48650&token=$TOKEN&pc=$PC&name=Pixel"

        private val vectors: JSONArray = run {
            val f = File("../../../docs/pairing-uri-vectors.json")
            assertTrue("arquivo de vetores não encontrado: rode a partir da raiz do repo", f.exists())
            JSONArray(f.readText(Charsets.UTF_8))
        }
    }

    // Os 16 vetores canônicos, 1:1 com `expect`/`value`.
    @Test
    fun ut203_allVectors() {
        assertEquals("quantidade de vetores", 16, vectors.length())
        for (i in 0 until vectors.length()) {
            val v = vectors.getJSONObject(i)
            val name = v.getString("name")
            val ctx = "vetor[$name]"
            val expect = v.getString("expect")
            val r = PairingUri.parse(v.getString("uri"))
            if (expect == "ok") {
                val p = r.getOrNull() ?: error("$ctx deveria ser ok, veio ${r.exceptionOrNull()}")
                val e = v.getJSONObject("value")
                assertEquals("$ctx v", e.getInt("v"), p.v)
                val hosts = e.getJSONArray("hosts")
                assertEquals(
                    "$ctx hosts",
                    (0 until hosts.length()).map { hosts.getString(it) },
                    p.hosts,
                )
                assertEquals("$ctx port", e.getInt("port"), p.port)
                assertEquals("$ctx token", e.getString("token"), p.token)
                assertEquals("$ctx pc", e.getString("pc"), p.pc)
                assertEquals("$ctx name", e.getString("name"), p.name)
            } else {
                assertTrue("$ctx deveria falhar, veio ok: ${r.getOrNull()}", r.isFailure)
                assertEquals("$ctx kind do erro", expect, (r.exceptionOrNull() as? UriError)?.kind)
            }
        }
    }

    // --- Casos extras do UT-2.03 (cartão F2-T05a) ---

    @Test
    fun ut203_portZeroAndTooHigh() {
        expectError(BASE.replace("port=48650", "port=0"), UriError.Invalid.kind, "port=0")
        expectError(BASE.replace("port=48650", "port=70000"), UriError.Invalid.kind, "port=70000")
    }

    @Test
    fun ut203_portMissingDefaultsTo48650() {
        expectOk(BASE.replace("&port=48650", ""))
    }

    @Test
    fun ut203_hostsEdgeCases() {
        expectError(BASE.replace("hosts=192.168.0.10", "hosts="), UriError.Invalid.kind, "hosts vazio")
        expectError(BASE.replace("hosts=192.168.0.10", "hosts=fe80::1"), UriError.Invalid.kind, "hosts IPv6")
        val six = BASE.replace(
            "hosts=192.168.0.10",
            "hosts=1.1.1.1,2.2.2.2,3.3.3.3,4.4.4.4,5.5.5.5,6.6.6.6",
        )
        expectError(six, UriError.Invalid.kind, "6 hosts")
    }

    @Test
    fun ut203_versionTooNew() {
        expectError(BASE.replace("v=1", "v=2"), UriError.VersionTooNew.kind, "v=2")
    }

    @Test
    fun ut203_tokenShortMissingUppercase() {
        expectError(
            BASE.replace("token=$TOKEN", "token=${TOKEN.dropLast(1)}"),
            UriError.Invalid.kind,
            "token 31 chars",
        )
        expectError(BASE.replace("&token=$TOKEN", ""), UriError.Invalid.kind, "sem token")
        expectError(
            BASE.replace("token=$TOKEN", "token=${TOKEN.uppercase()}"),
            UriError.Invalid.kind,
            "token maiúsculo",
        )
    }

    @Test
    fun ut203_pcAndNameOptionalOrValid() {
        expectOk(BASE.replace("&pc=$PC", ""), expectPc = null)
        expectOk(BASE.replace("&name=Pixel", ""), expectName = null)
        expectError(BASE.replace("pc=$PC", "pc="), UriError.Invalid.kind, "pc vazio")
        expectError(BASE.replace("pc=$PC", "pc=nao-e-uuid"), UriError.Invalid.kind, "pc não-UUID")
        expectError(BASE.replace("name=Pixel", "name="), UriError.Invalid.kind, "name vazio")
    }

    @Test
    fun ut203_unknownParamsIgnored() {
        expectOk(BASE + "&foo=bar&baz=42")
    }

    // --- Helpers ---

    private fun expectOk(uri: String, expectPc: String? = PC, expectName: String? = "Pixel") {
        val r = PairingUri.parse(uri)
        val p = r.getOrNull() ?: error("deveria ser ok, veio ${r.exceptionOrNull()}")
        assertEquals("v", 1, p.v)
        assertEquals("hosts", listOf("192.168.0.10"), p.hosts)
        assertEquals("port", 48650, p.port)
        assertEquals("token", TOKEN, p.token)
        assertEquals("pc", expectPc, p.pc)
        assertEquals("name", expectName, p.name)
    }

    private fun expectError(uri: String, kind: String, ctx: String) {
        val r = PairingUri.parse(uri)
        assertTrue("$ctx: deveria falhar, veio ok: ${r.getOrNull()}", r.isFailure)
        assertEquals("$ctx: kind do erro", kind, (r.exceptionOrNull() as? UriError)?.kind)
    }
}
