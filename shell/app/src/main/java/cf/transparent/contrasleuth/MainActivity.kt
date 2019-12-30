package cf.transparent.contrasleuth

import android.content.Context
import androidx.appcompat.app.AppCompatActivity
import android.os.Bundle
import android.util.Log
import android.webkit.WebView
import java.io.*

class MainActivity : AppCompatActivity() {

    @Throws(IOException::class)
    fun getCacheFile(context: Context, filename: String): File = File(context.cacheDir, filename)
        .also {
            it.outputStream().use { cache -> context.assets.open(filename).use { it.copyTo(cache) } }
        }

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        setContentView(R.layout.activity_main)

        val webView: WebView = findViewById(R.id.webview)
        webView.loadUrl("file:///android_asset/index.html")
        // ====== Would you like to execute a child process? =====
        val executable = getCacheFile(this, "contrasleuth.elf")
        executable.setExecutable(true)
        Log.e("ready", executable.path)
        val sh = Runtime.getRuntime().exec(executable.path)
        val outputStream = DataOutputStream(sh.outputStream)
        val bufferedInputStream = BufferedReader(InputStreamReader(sh.inputStream))
        Log.e("WHAT?", "LOGGED")
        while (true) {
            val line = bufferedInputStream.readLine()
            if (line == null) break
            Log.e("TAG", line)
        }
    }
}
