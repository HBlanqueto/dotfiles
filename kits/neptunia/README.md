# Neptunia: diseños guardados como datos

Esta carpeta es la única fuente del diseño de nixdots. Los lienzos de Claude
son donde se diseña y se aprueba; aquí se guarda lo aprobado en un formato que
cualquier proyecto puede leer, presente o futuro, sin depender de un framework.

```
kits/neptunia/
├── tokens.json        colores (claro, oscuro, oscuro tintado), radios, espacios, tamaños,
│                      duraciones, tipografía y sombras
├── components.json    medidas y estados de cada componente aprobado (interruptor 32×18,
│                      segmentado rectangular, pestañas de barra de herramientas…)
├── default.nix        traduce los JSON y compone el tema; solo builtins, sin nixpkgs
├── neptunia.nix       paleta de terminal (colors)
├── filecolors.nix     colores por tipo de archivo (eza)
├── generated/         salidas ya generadas, para quien no use Nix (SlintPad, otro repo)
│   ├── neptunia.slint   globals Theme y Metrics para Slint
│   ├── neptunia.css     variables CSS (GTK4, web)
│   └── Neptunia.qml     singleton QtQuick
└── demo.slint         galería para ajustar a ojo con slint-viewer o SlintPad
```

## Regla de oro

Nadie escribe un color ni una medida a mano. Si cambia el diseño, cambia
`tokens.json` o `components.json` y se regenera:

```sh
cd kits/neptunia
nix eval --raw -f . slint > generated/neptunia.slint
nix eval --raw -f . css   > generated/neptunia.css
nix eval --raw -f . qml   > generated/Neptunia.qml
```

## Modos de color del escritorio

El bloque `appearance` de `components.json` describe (como idea en estudio) el
color de las ventanas: `neutral` (sin tinte), `accent` (tintado por el acento) y
`wallpaper` (tintado por el color dominante del fondo, en toda la ventana o en
todo el escritorio). El mismo bloque elige el estilo de los semáforos —`macos`
(colores fijos), `accent` o `wallpaper` (derivados de su color)—; los `sys-*`
(avisos, error, éxito…) nunca se tintan y `layer-factors` fija cuánto se aclara
u oscurece cada capa.

## Valores de la shell y del compositor

| Token / medida | Valor | Uso |
| --- | --- | --- |
| `traffic-close` / `traffic-min` / `traffic-zoom` | `#ff5f57` / `#febc2e` / `#28c840` | Semáforos estilo macOS, iguales en los tres temas |
| `traffic-inactive` | `#d3d3d5` claro; `#4e4e50` oscuro y dark-tint | Semáforos sin foco |
| `traffic-rim` | `#00000033` | Aro de 0,5 px |
| `traffic-glyph` | `#0000008c` | Glifo de los semáforos |
| `titlebar-bg` / `titlebar-bg-inactive` | `bg-window` / `bg-sunken` | Barra de título |
| `titlebar-title` / `titlebar-title-inactive` | `ink-strong` / `ink-muted` | Texto de la barra |
| `titlebar-line` | `line` | Línea de la barra |
| `size.titlebar-h` | `44` | Barra de las apps propias |
| `size.titlebar-ssd-h` | `32` | Barra que dibuja el compositor para apps de terceros con decoración del servidor |
| `font.families.sans` | `Geist, "Noto Sans CJK SC", system-ui, sans-serif` | Familia de la interfaz |
| `font.styles.titlebar-title` | 13 px, 17 px de línea, 600 | Título de la barra |
| `font.weights` | 400, 500, 600, 650, 700 | Pesos usados |
| `shell-bar` | `#f2f4f7e6` / `#1e1e1ee6` / `#1d1e20e6` | Fondo translúcido de menubar/dock |
| `shell.menubar` | 28 px; 13/500; margen 10; `shell-bar` / `ink-strong` | Menubar |
| `shell.dock` | tile 48, gap 6, padding 8, tile-radius 16, radius 24, indicator 4; `bg-control` / `accent` / `on-accent` | Dock |
| `shell.lock` | título 24/600; campo `text-field`; `scrim` | Lock |
| `shadow.layers.window` | light 48/18/0.18 + 12/4/0.08; dark y dark-tint 56/22/0.55 + 16/6/0.35 | Sombra de ventana por capas |
| `shadow.layers.window.unfocused-scale` | `0.5` | Sombra sin foco |
| `shadow.layers.float` | light 28/10/0.16; dark y dark-tint 32/12/0.50 | Sombra flotante |

**32 px es la barra del compositor para apps de terceros; 44 px es la de las
apps propias.**

Los colores con alfa se guardan como `#rrggbbaa` en los JSON, Slint y CSS, y
como `#aarrggbb` en QML.

## Cómo lo lee cada proyecto

| Proyecto | Cómo se sincroniza |
| --- | --- |
| Psamathe y apps Triton (Slint) | `import { Theme, Metrics } from "…/neptunia.slint";`. Desde Nix: `(import ./kits/neptunia).slint`; o copia `generated/neptunia.slint`. Cambiar de tema es `Theme.mode = NeptuniaMode.dark;`. |
| Instalador (ratatui) | Sigue con la paleta de terminal de `kits/neptunia/neptunia.nix`. Si algún día quiere los grises de interfaz: `serde_json` sobre `tokens.json` con `include_str!`. |
| GTK4 / web | `neptunia.css` con `data-theme="dark"` o `"dark-tint"` en la raíz. |
| Qt / QML | `generated/Neptunia.qml` como singleton; los alfa van en `#aarrggbb`. |
| Home Manager | `xdg.configFile."neptunia".source = (import ./kits/neptunia).files pkgs;` deja los archivos generados y los JSON en `~/.config/neptunia/`, y cualquier app los lee de ahí. |
| Otro repo futuro | Como input del flake (`inputs.nixdots`) y `import "${inputs.nixdots}/kits/neptunia"`, o leyendo `tokens.json` directamente. Cada proyecto genera lo suyo a su manera; los datos son los mismos. |

## Comprobar las salidas

El flake compara `generated/` con lo que produce `default.nix` y compila
`demo.slint` con `slint-viewer` bajo Xvfb:

```sh
nix flake check
# o, para regenerar:
cd kits/neptunia
nix eval --raw -f . slint > generated/neptunia.slint
nix eval --raw -f . css   > generated/neptunia.css
nix eval --raw -f . qml   > generated/Neptunia.qml
```

## Ajustar una pantalla sin recompilar Rust

`slint-viewer --auto-reload` abre un `.slint` suelto y lo recarga cada vez que
guardas. Es lo más rápido para ajustar una sola pantalla, por ejemplo «Tu
cuenta», sin tocar el resto:

```sh
slint-viewer --auto-reload kits/neptunia/demo.slint
slint-viewer --auto-reload setup-gui/ui/cuenta.slint   # una pantalla concreta
```

`slint-lsp`, con la extensión de VS Code o con Neovim o Helix, da
autocompletado y una vista previa en vivo en la que se pueden tocar
propiedades. Los dos se instalan desde nixpkgs (`slint-lsp`, `slint-viewer`)
en el `devShell`.

## SlintPad

SlintPad (slintpad.com) es un editor en el navegador para probar un componente
sin instalar nada. Como `demo.slint` importa `generated/neptunia.slint`, hay dos
formas de abrirlo:

- Pega primero el contenido de `generated/neptunia.slint` y debajo el
  componente que quieras probar, quitando la línea `import`.
- O ábrelo desde GitHub con
  `https://slintpad.com/?load_url=<url raw de demo.slint>`. SlintPad suele
  resolver el import relativo; si no lo resuelve, usa la primera forma.

## Traducciones con `@tr()`

Marca cada texto visible con `@tr("Continuar")`, como ya hace `demo.slint`, y
usa el español como idioma fuente. Slint extrae esas cadenas a archivos `.po`,
uno por idioma, que se compilan dentro del binario. Así el asistente puede
cambiar de idioma en vivo cuando se elige en el paso 1:

```sh
cargo install slint-tr-extractor
find setup-gui/ui -name '*.slint' | xargs slint-tr-extractor -o psamathe.pot
msginit -i psamathe.pot -l en -o lang/en/LC_MESSAGES/psamathe.po
```

```rust
// build.rs (versiones recientes de Slint; comprueba la API de la versión que uses)
slint_build::compile_with_config(
    "ui/app.slint",
    slint_build::CompilerConfiguration::new().with_bundled_translations("lang"),
).unwrap();
// al elegir el idioma:
slint::select_bundled_translation("en").ok();
```

El saludo animado en 13 idiomas no es una traducción: es una lista fija de
palabras y se queda como datos.

## De los lienzos de Claude a esta carpeta

1. Se diseña y se aprueba en los lienzos (sistema de diseño Neptunia).
2. Lo aprobado se vuelca a `tokens.json` y `components.json`. Pide a Claude
   «actualiza kits/neptunia» y lo hace desde el sistema de diseño.
3. Se regeneran las salidas y cada app las toma en su siguiente build.
