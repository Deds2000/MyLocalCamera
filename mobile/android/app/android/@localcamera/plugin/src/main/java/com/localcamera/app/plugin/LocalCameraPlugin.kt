// STUB DE REGISTRO — NÃO COMPILADO.
//
// Este arquivo está fora dos source sets do app (nem `app/src/main/java` nem
// o project Gradle `:localcamera-plugin`, que não tem build.gradle), então o
// Kotlin/AGP nunca o compilam. Ele existe apenas para o `cap sync`
// (@capacitor/cli, `dist/android/update.js` → `findAndroidPluginClassesInPlugin`):
// o CLI escaneia este diretório em busca da anotação `@CapacitorPlugin` e
// grava o classpath em `app/src/main/assets/capacitor.plugins.json`, que o
// Bridge carrega em tempo de execução.
//
// A classe REAL (mesmo FQCN) vive no app:
//   ../../../src/main/java/com/localcamera/app/plugin/LocalCameraPlugin.kt
//
// Mantenha a anotação e o nome da classe idênticos aos da classe real.

package com.localcamera.app.plugin

import com.getcapacitor.Plugin
import com.getcapacitor.annotation.CapacitorPlugin

@CapacitorPlugin(name = "LocalCamera")
class LocalCameraPlugin : Plugin()
