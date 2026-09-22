# NOVA Desktop

App de escritorio (Tauri) que abre NOVA en una ventana nativa, apuntando directo al servidor —
sin que el usuario tenga que escribir la IP en el navegador.

No contiene lógica de negocio: toda la app sigue viviendo en el servidor PHP
(`/var/www/fmt` en el repo `PLATAFORMA-OMAS`). Esto es solo el "shell" nativo.

## Configuración

La URL del servidor está en `src-tauri/tauri.conf.json` → `app.windows[0].url`.
Hoy apunta a `http://192.168.10.25/` (IP LAN actual). Si el servidor cambia de IP,
o si se migra a un hostname/HTTPS interno, actualizar ese valor ahí.

## Desarrollo local

Requiere Rust (`rustup`) instalado además de Node. Luego:

```bash
npm install
npm run tauri dev
```

## Generar el instalador de Windows (.exe / .msi)

No se puede compilar el `.exe` desde Linux de forma confiable. Dos opciones:

1. **Recomendado**: crear un tag (`git tag v0.1.0 && git push --tags`) — el workflow
   `.github/workflows/build.yml` compila en un runner `windows-latest` y deja el
   instalador como asset en una release (borrador) del repo.
2. Compilar a mano en una máquina Windows con Rust + WebView2 instalados: `npm install && npm run tauri build`.

## Pendiente / próximos pasos

- Definir hostname estable (o HTTPS interno) en vez de la IP cruda, para que la app
  no se rompa si el servidor cambia de IP por DHCP.
- Icono/branding definitivo en `src-tauri/icons/` (hoy son los de ejemplo de Tauri).
- Certificado de firma de código, si se quiere evitar el aviso de SmartScreen en Windows.
