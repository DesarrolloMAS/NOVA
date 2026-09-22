use std::sync::atomic::{AtomicU32, Ordering};
use tauri::{WebviewUrl, WebviewWindowBuilder};
use tauri_plugin_updater::UpdaterExt;

const SERVER_URL: &str = "http://192.168.10.25/";

// Los PDFs (y otros enlaces) de NOVA se abren con target="_blank" / window.open(),
// que un webview embebido no maneja como un navegador normal (no crea ventana nueva
// y la apertura se pierde en silencio). Este script se inyecta en cada página cargada
// e intercepta ambos casos y llama al comando `open_viewer_window` (ver abajo) en vez
// de dejar que se pierda — una ventana nueva DE LA APP comparte sesión/cookies con la
// principal (mismo perfil de WebView2), así el PDF ya sale logueado.
const OPEN_EXTERNAL_SCRIPT: &str = r#"
(function () {
  function openInAppWindow(url) {
    if (!url) return;
    if (window.__TAURI_INTERNALS__) {
      window.__TAURI_INTERNALS__.invoke('open_viewer_window', { url: url });
    }
  }
  window.open = function (url) { openInAppWindow(url); return null; };
  document.addEventListener('click', function (e) {
    var a = e.target.closest && e.target.closest('a[target="_blank"]');
    if (a && a.href) {
      e.preventDefault();
      openInAppWindow(a.href);
    }
  }, true);
})();
"#;

static VIEWER_WINDOW_COUNTER: AtomicU32 = AtomicU32::new(0);

#[tauri::command]
fn open_viewer_window(app: tauri::AppHandle, url: String) -> Result<(), String> {
    let parsed = url.parse().map_err(|e| format!("URL inválida: {e}"))?;
    let n = VIEWER_WINDOW_COUNTER.fetch_add(1, Ordering::SeqCst);
    let label = format!("viewer-{n}");
    WebviewWindowBuilder::new(&app, label, WebviewUrl::External(parsed))
        .title("NOVA - Documento")
        .inner_size(1000.0, 800.0)
        .build()
        .map_err(|e| e.to_string())?;
    Ok(())
}

// Revisa si hay una versión nueva publicada y, si la hay, la descarga, instala y
// reinicia la app. Se corre en background al arrancar — silencioso mientras no hay
// update (no molesta al usuario en el día a día).
async fn check_for_updates(app: tauri::AppHandle) {
    let updater = match app.updater() {
        Ok(updater) => updater,
        Err(e) => {
            eprintln!("Updater no disponible: {e}");
            return;
        }
    };

    match updater.check().await {
        Ok(Some(update)) => {
            println!("Actualización disponible: {}", update.version);
            if let Err(e) = update.download_and_install(|_chunk, _total| {}, || {}).await {
                eprintln!("Error instalando la actualización: {e}");
                return;
            }
            app.restart();
        }
        Ok(None) => {}
        Err(e) => eprintln!("Error revisando actualizaciones: {e}"),
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .invoke_handler(tauri::generate_handler![open_viewer_window])
        .setup(|app| {
            WebviewWindowBuilder::new(app, "main", WebviewUrl::External(SERVER_URL.parse()?))
                .title("NOVA")
                .inner_size(1280.0, 800.0)
                .min_inner_size(900.0, 600.0)
                .initialization_script(OPEN_EXTERNAL_SCRIPT)
                .build()?;

            let handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                check_for_updates(handle).await;
            });

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
