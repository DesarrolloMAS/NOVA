use std::sync::atomic::{AtomicU32, Ordering};
use tauri::{Manager, WebviewUrl, WebviewWindowBuilder};
use tauri_plugin_updater::UpdaterExt;

const SERVER_URL: &str = "http://192.168.10.25/";

// Los PDFs (y otros enlaces) de NOVA se abren con target="_blank" / window.open(),
// que un webview embebido no maneja como un navegador normal (no crea ventana nueva
// y la apertura se pierde en silencio). Este script se inyecta en cada página cargada
// e intercepta ambos casos y llama al comando `open_viewer_window` (ver abajo) en vez
// de dejar que se pierda — una ventana nueva DE LA APP comparte sesión/cookies con la
// principal (mismo perfil de WebView2), así el PDF ya sale logueado.
//
// OJO: algunos módulos llaman window.open() con una ruta RELATIVA (ej. "visor_x.php?..",
// sin "http://ip/..."), a diferencia de los <a target="_blank"> donde `.href` siempre
// devuelve la URL ya resuelta por el navegador. Por eso se resuelve con `new URL(url, base)`
// antes de mandarla a Rust — si no, `Url::parse` del lado de Rust falla (no es absoluta)
// y la promesa de invoke() queda rechazada en silencio, sin que se vea nada en pantalla.
const OPEN_EXTERNAL_SCRIPT: &str = r#"
(function () {
  function resolve(url) {
    try { return new URL(url, window.location.href).toString(); } catch (e) { return url; }
  }
  function openViewer(url) {
    if (!url) return;
    var abs = resolve(url);
    if (!window.__TAURI_INTERNALS__) return;
    window.__TAURI_INTERNALS__.invoke('open_viewer_window', { url: abs }).catch(function (err) {
      // DIAGNÓSTICO TEMPORAL: mostrar el error real en pantalla para saber por qué
      // falla la ventana nativa (se quita en cuanto tengamos la causa confirmada).
      try { alert('open_viewer_window falló: ' + JSON.stringify(err)); } catch (e2) {}
      // Respaldo: al menos que abra en el navegador del sistema en vez de no hacer nada.
      window.__TAURI_INTERNALS__.invoke('plugin:opener|open_url', { url: abs });
    });
  }
  window.open = function (url) { openViewer(url); return null; };
  document.addEventListener('click', function (e) {
    var a = e.target.closest && e.target.closest('a[target="_blank"]');
    if (a && a.href) {
      e.preventDefault();
      openViewer(a.href);
    }
  }, true);
})();
"#;

// Insignia con la versión instalada, para confirmar a simple vista que una
// actualización se aplicó. Se inyecta solo en la ventana principal (login incluido).
const VERSION_BADGE_TEMPLATE: &str = r#"
(function () {
  var b = document.createElement('div');
  b.textContent = 'NOVA v__APP_VERSION__';
  b.style.cssText = 'position:fixed;bottom:6px;right:8px;z-index:2147483647;' +
    'background:rgba(0,0,0,.6);color:#fff;font:11px monospace;padding:2px 6px;' +
    'border-radius:4px;pointer-events:none;';
  function mount() { if (document.body) document.body.appendChild(b); }
  if (document.readyState === 'loading') {
    document.addEventListener('DOMContentLoaded', mount);
  } else {
    mount();
  }
})();
"#;

static VIEWER_WINDOW_COUNTER: AtomicU32 = AtomicU32::new(0);

// async: si esto corriera sincrónico, se traba en Windows — crear una ventana
// dentro de un comando síncrono se autobloquea porque el comando corre en el
// mismo hilo principal que necesita usar internamente para crear la ventana.
#[tauri::command]
async fn open_viewer_window(app: tauri::AppHandle, url: String) -> Result<(), String> {
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
            let version = app.package_info().version.to_string();
            let badge_script = VERSION_BADGE_TEMPLATE.replace("__APP_VERSION__", &version);
            let init_script = format!("{OPEN_EXTERNAL_SCRIPT}\n{badge_script}");

            WebviewWindowBuilder::new(app, "main", WebviewUrl::External(SERVER_URL.parse()?))
                .title(format!("NOVA v{version}"))
                .inner_size(1280.0, 800.0)
                .min_inner_size(900.0, 600.0)
                .initialization_script(init_script)
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
