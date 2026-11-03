package com.localcamera.app.plugin

import android.content.Context
import com.getcapacitor.JSObject
import com.getcapacitor.PluginCall
import com.localcamera.app.store.PairingStore
import org.json.JSONArray
import org.json.JSONObject

/** Métodos Capacitor para expor apenas metadados de PCs com chave persistida. */
internal object KnownPcPluginBridge {

    fun getKnownPcs(context: Context, call: PluginCall, log: (String) -> Unit) {
        try {
            val store = PairingStore(context)
            val pcs = JSONArray()
            store.listPcs().forEach { pc ->
                if (store.deviceKey(pc.pcId) == null) return@forEach
                val hosts = JSONArray().apply { pc.hosts.forEach { put(it) } }
                pcs.put(
                    JSONObject()
                        .put(KEY_PC_ID, pc.pcId)
                        .put(KEY_NAME, pc.name)
                        .put(KEY_HOSTS, hosts)
                        .put(KEY_PORT, pc.port)
                        .put(KEY_LAST_ACCESSED, pc.lastAccessed ?: JSONObject.NULL),
                )
            }
            call.resolve(JSObject().put(KEY_PCS, pcs))
        } catch (e: Exception) {
            log("lista de PCs conhecidos falhou (${e::class.simpleName})")
            call.reject(ERROR_PC_STORE)
        }
    }

    fun forgetPc(context: Context, call: PluginCall, log: (String) -> Unit) {
        val pcId = call.getString(KEY_PC_ID)
        if (pcId == null || !PC_ID_RE.matches(pcId)) {
            call.reject(ERROR_INVALID_ARGS)
            return
        }
        try {
            PairingStore(context).forgetPc(pcId)
            call.resolve()
        } catch (e: Exception) {
            log("esquecimento de PC falhou (${e::class.simpleName})")
            call.reject(ERROR_PC_STORE)
        }
    }

    private const val KEY_PCS = "pcs"
    private const val KEY_PC_ID = "pcId"
    private const val KEY_NAME = "name"
    private const val KEY_HOSTS = "hosts"
    private const val KEY_PORT = "port"
    private const val KEY_LAST_ACCESSED = "lastAccessed"
    private const val ERROR_INVALID_ARGS = "invalid_args"
    private const val ERROR_PC_STORE = "pc_store_failed"
    private val PC_ID_RE = Regex("^[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}$")
}
