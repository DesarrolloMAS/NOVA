use tauri::{WebviewUrl, WebviewWindowBuilder};
use tauri_plugin_updater::UpdaterExt;

const SERVER_URL: &str = "http://192.168.10.25/";

// Los PDFs (y otros enlaces) de NOVA se abren con target="_blank" / window.open(),
// que un webview embebido no maneja como un navegador normal (no crea ventana nueva
// y la apertura se pierde en silencio). Este script se inyecta en cada página cargada
// e intercepta ambos casos para abrirlos en el navegador por defecto del sistema.
const OPEN_EXTERNAL_SCRIPT: &str = r#"
(function () {
  function openExternal(url) {
    if (!url) return;
    if (window.__TAURI_INTERNALS__) {
      window.__TAURI_INTERNALS__.invoke('plugin:opener|open_url', { url: url });
    }
  }
  window.open = function (url) { openExternal(url); return null; };
  document.addEventListener('click', function (e) {
    var a = e.target.closest && e.target.closest('a[target="_blank"]');
    if (a && a.href) {
      e.preventDefault();
      openExternal(a.href);
    }
  }, true);
})();
"#;

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
