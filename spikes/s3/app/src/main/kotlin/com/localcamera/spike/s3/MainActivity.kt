package com.localcamera.spike.s3

import android.Manifest
import android.content.pm.PackageManager
import android.os.Bundle
import android.util.TypedValue
import android.view.Gravity
import android.view.View
import android.view.WindowManager
import android.widget.Button
import android.widget.LinearLayout
import androidx.activity.result.contract.ActivityResultContracts
import androidx.appcompat.app.AppCompatActivity
import androidx.core.content.ContextCompat

/**
 * Spike S3 — app descartável.
 * 4 botões (T1..T4) + Parar. Automação via extras:
 *   adb shell am start -n com.localcamera.spike.s3/.MainActivity --es test T1 --ei seconds 600
 * seconds <= 0 => 600 (10 min).
 */
class MainActivity : AppCompatActivity() {

    private var currentTest: SpikeTest? = null
    private var pendingTestId: String? = null
    private var autoSeconds: Long = DEFAULT_SECONDS

    private val permLauncher =
        registerForActivityResult(ActivityResultContracts.RequestPermission()) { granted ->
            val id = pendingTestId
            pendingTestId = null
            if (granted) {
                id?.let { startTestById(it, autoSeconds) }
            } else {
                S3Log.e("error=camera_permission_denied test=$id")
            }
        }

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        // Spike de coleta: mantém a tela acordada durante os testes de estabilidade.
        // (o teste de "tela apagando" é um passo humano separado)
        window.addFlags(WindowManager.LayoutParams.FLAG_KEEP_SCREEN_ON)
        setContentView(buildUi())
        S3Log.i("app_started pkg=$packageName")

        val testExtra = intent.getStringExtra(EXTRA_TEST)
        val secondsExtra = intent.getIntExtra(EXTRA_SECONDS, 0)
        if (!testExtra.isNullOrBlank() && testExtra.uppercase() in SUPPORTED_TESTS) {
            autoSeconds = if (secondsExtra > 0) secondsExtra.toLong() else DEFAULT_SECONDS
            ensurePermissionAndStart(testExtra.uppercase())
        }
    }

    private fun buildUi(): View {
        val root = LinearLayout(this).apply {
            orientation = LinearLayout.VERTICAL
            gravity = Gravity.CENTER
            val pad = dp(16)
            setPadding(pad, pad, pad, pad)
        }
        for (id in TEST_ORDER) {
            root.addView(makeButton(id) {
                ensurePermissionAndStart(id)
            })
        }
        root.addView(makeButton("PARAR") {
            currentTest?.stop()
        })
        return root
    }

    private fun makeButton(text: String, onClick: () -> Unit): Button = Button(this).apply {
        this.text = text
        isAllCaps = false
        textSize = 16f
        TypedValue.applyDimension(
            TypedValue.COMPLEX_UNIT_DIP, 48f, resources.displayMetrics
        ).let { minHeight = it.toInt() }
        val lp = LinearLayout.LayoutParams(
            LinearLayout.LayoutParams.MATCH_PARENT,
            LinearLayout.LayoutParams.WRAP_CONTENT
        ).apply { topMargin = dp(8) }
        layoutParams = lp
        setOnClickListener { onClick() }
    }

    private fun dp(v: Int): Int =
        TypedValue.applyDimension(
            TypedValue.COMPLEX_UNIT_DIP, v.toFloat(), resources.displayMetrics
        ).toInt()

    private fun ensurePermissionAndStart(testId: String) {
        if (ContextCompat.checkSelfPermission(this, Manifest.permission.CAMERA)
            == PackageManager.PERMISSION_GRANTED
        ) {
            startTestById(testId, DEFAULT_SECONDS)
        } else {
            pendingTestId = testId
            permLauncher.launch(Manifest.permission.CAMERA)
        }
    }

    private fun startTestById(testId: String, seconds: Long) {
        currentTest?.stop()
        val t: SpikeTest = when (testId) {
            "T1" -> T1CameraXTest(this, seconds)
            "T2" -> T2Camera2Test(this, seconds)
            "T3" -> T3Fps60Test(this, seconds)
            "T4" -> T4DualTest(this, seconds)
            else -> {
                S3Log.e("error=unknown_test $testId")
                return
            }
        }
        currentTest = t
        t.start()
    }

    override fun onStop() {
        super.onStop()
        currentTest?.stop()
    }

    companion object {
        const val EXTRA_TEST = "test"
        const val EXTRA_SECONDS = "seconds"
        const val DEFAULT_SECONDS = 600L
        val TEST_ORDER = listOf("T1", "T2", "T3", "T4")
        val SUPPORTED_TESTS = TEST_ORDER
    }
}
