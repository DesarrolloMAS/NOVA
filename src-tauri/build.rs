fn main() {
    // Registra open_viewer_window para que tauri-build genere el permiso
    // "allow-open-viewer-window" — sin esto, el ACL rechaza el comando cuando
    // se invoca desde una ventana con contenido remoto (nuestro caso: la ventana
    // principal carga el servidor real, no un origen local empaquetado).
    let attributes = tauri_build::Attributes::new()
        .app_manifest(tauri_build::AppManifest::new().commands(&["open_viewer_window"]));
    tauri_build::try_build(attributes).expect("failed to run tauri-build");
}
