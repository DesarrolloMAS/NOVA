# NOVA Desktop

App de escritorio (Tauri) que abre NOVA en una ventana nativa, apuntando directo al servidor —
sin que el usuario tenga que escribir la IP en el navegador.

No contiene lógica de negocio: toda la app sigue viviendo en el servidor PHP
(`/var/www/fmt` en el repo `PLATAFORMA-OMAS`). Esto es solo el "shell" nativo.

## Configuración

La URL del servidor está en `src-tauri/src/lib.rs` → constante `SERVER_URL`.
Hoy apunta a `http://192.168.10.25/` (IP LAN actual). Si el servidor cambia de IP,
o si se migra a un hostname/HTTPS interno, actualizar ese valor ahí **y** el patrón
`remote.urls` en `src-tauri/capabilities/default.json`.

La ventana ya no se declara en `tauri.conf.json` (quedó `"windows": []`): se crea a mano
en `setup()` (`lib.rs`) porque necesita un script inyectado que intercepta los enlaces
`target="_blank"`/`window.open()` (así es como la plataforma abre los PDFs). Esos enlaces
se abren en **una ventana nueva de la misma app** (comando `open_viewer_window`), no en
el navegador del sistema — así comparte sesión/cookies con la ventana principal (mismo
perfil de WebView2) y no pide login de nuevo. De paso, esa ventana no tiene barra de
direcciones, así que tampoco se ve la IP del servidor.

## Desarrollo local

Requiere Rust (`rustup`) instalado además de Node. Luego:

```bash
npm install
npm run tauri dev
```

## Generar el instalador de Windows (.exe / .msi)

No se puede compilar el `.exe` desde Linux de forma confiable. Dos opciones:

1. **Recomendado**: crear un tag (`git tag v0.1.0 && git push --tags`) — el workflow
   `.github/workflows/build.yml` compila en un runner `windows-latest` y publica el
   instalador como asset de una release del repo (ver más abajo: para que el
   auto-update funcione, la release NO debe quedar en modo borrador).
2. Compilar a mano en una máquina Windows con Rust + WebView2 instalados: `npm install && npm run tauri build`.

## Sistema de actualizaciones automáticas

Ya está cableado en el código (`tauri-plugin-updater` + `tauri-plugin-process` en
`Cargo.toml`, chequeo al arrancar en `lib.rs::check_for_updates`, config en
`tauri.conf.json` → `plugins.updater`), pero **no queda activo hasta que exista el
repo en GitHub**, porque depende de publicar releases ahí:

- `plugins.updater.endpoints` en `tauri.conf.json` apunta a
  `https://github.com/DesarrolloMAS/NOVA/releases/latest/download/latest.json`.
- El par de llaves de firma del updater (distinto de la firma de código de Windows)
  ya se generó en `.keys/nova-desktop-updater.key` (privada, sin password, **nunca
  se sube a git** — está en `.gitignore`) y `.keys/nova-desktop-updater.key.pub`
  (pública, ya copiada dentro de `tauri.conf.json`).
- Antes de que el workflow de CI pueda firmar releases, hay que cargar la llave
  privada como **secreto del repo** en GitHub (Settings → Secrets and variables →
  Actions):
  - `TAURI_SIGNING_PRIVATE_KEY`: contenido completo de `.keys/nova-desktop-updater.key`
  - `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`: dejar vacío (la llave no tiene password)
- Con eso puesto, cada tag/release nueva deja un `latest.json` firmado como asset,
  y la app lo revisa sola al arrancar — si hay versión nueva, la descarga, instala
  y se reinicia sola, sin que el usuario haga nada.

## Pendiente / próximos pasos

- Cargar los dos secrets de arriba en el repo (Settings → Secrets and variables →
  Actions) — bloquea todo lo del updater y el CI hasta que estén puestos.
- Definir hostname estable (o HTTPS interno) en vez de la IP cruda, para que la app
  no se rompa si el servidor cambia de IP por DHCP.
- Icono/branding definitivo en `src-tauri/icons/` (hoy son los de ejemplo de Tauri).
- Certificado de firma de código, si se quiere evitar el aviso de SmartScreen en Windows
  (esto es aparte de la llave del updater — son dos firmas con propósitos distintos).
