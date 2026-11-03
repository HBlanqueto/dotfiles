# nixdots-installer

Un instalador de terminal («siguiente, siguiente, aceptar») para la
configuración NixOS de este repositorio. Es un ejecutable Rust que **orquesta**
herramientas existentes — `disko`, `nixos-facter`, `sbctl`, `nix`,
`nixos-install`, `nmcli`… — y nunca reimplementa el particionado.

## Simulación

Sin argumentos, el instalador recorre todas sus pantallas, muestra los comandos
que ejecutaría y no toca ningún disco. Funciona en cualquier Linux, sin root:

```sh
nix run .#installer                 # --repo <ruta|url> para elegir el repo
```

En la cabecera aparece `simulación`. Pulsa `v` para mostrar la columna de
comandos (y, durante la instalación, el log en vivo). La simulación tarda ~20 s
en instalar, para poder ver la pantalla completa.

## Instalación real (desde la ISO mínima de NixOS)

La vía normal no compila nada: descarga el binario precompilado de la última
release y lo ejecuta, añadiendo con `nix shell` las herramientas que la ISO no
trae (en una VT real abre antes `kmscon` con la fuente **JetBrainsMono Nerd Font
Mono 14** y la paleta de neptunia).

```sh
nix-shell -p git --run "git clone --depth 1 https://github.com/HBlanqueto/nixdots"
sudo ./nixdots/install.sh
```

Para desarrollo (compila desde el código):

```sh
nix --extra-experimental-features 'nix-command flakes' \
  run github:HBlanqueto/nixdots#installer -- --install
```

`--install` exige root y comprobar que se está ejecutando desde el entorno live
(`/etc/NIXOS`). El log completo queda en `/tmp/nixdots-install.log` y se copia a
`/mnt/persist/` si ese montaje existe.

Pantallas: Inicio · Licencia · Red · Lenguaje · Teclado · Zona horaria · Equipo
· Disco · Cifrado · Resumen · Instalando · Listo.

La cuenta (usuario, contraseña y git) **no** se pide aquí: la crea el asistente
de primer arranque la primera vez que se reinicia el equipo.

### Opciones

- `--install` — instalación real.
- `--repo <ruta|url>` — repositorio a instalar (por defecto
  `github:HBlanqueto/nixdots`, clonado a `/tmp/nixdots`).

## Atajos

- `tab` / `shift+tab` — moverse entre el contenido y los botones.
- `←`/`→` — elegir entre botones u opciones.
- `enter` — continuar / activar el botón enfocado.
- `v` — columna de comandos (en **Instalando**, `v log: on/off`).
- `F2` — selector flotante de distribución de teclado (salvo Inicio, Instalando
  y Listo).
- `ctrl+c` — cancelar (en **Instalando** abre la confirmación) o salir.
- `q` — salir.

El ratón no se usa para nada: la TUI no lo captura, así que no hay clics (para
seleccionar y copiar texto, la terminal sigue funcionando con normalidad).

## Red

La ISO mínima de NixOS activa `networking.networkmanager`, así que el instalador
usa `nmcli` (y recurre a `iwctl` solo si faltara). No se avanza sin
conectividad comprobada contra `cache.nixos.org`.

## Licencia

El texto GPL va incrustado (funciona sin red ni repositorio) y se desplaza
dentro del panel con `↑↓`, `PageUp/PageDown` o `Home/End`.
`espacio` (o `x`) marca **Acepto**; no se avanza sin aceptarla.

## Notas

- `disko` formatea la raíz como un contenedor **LUKS2** (`cryptroot`); la frase
  de desbloqueo se escribe en la pantalla **Cifrado** y se pasa a `disko`
  mediante un archivo temporal `0600` (`--argstr luksPasswordFile`), eliminado
  en cuanto termina. El primer arranque la pide.
- El TPM **no** se inscribe aquí: lo hace `lanzaboote` (`measuredBoot.autoCryptenroll`,
  vía `systemd-pcrlock`) en el primer arranque, de modo que la política se
  actualiza sola cuando cambian las PCR de Secure Boot.
- La paleta proviene de `kits/neptunia/neptunia.nix`; un test (`cargo test`) falla
  si las constantes de `src/theme.rs` se desincronizan.
- En la consola Linux (`TERM=linux`, la tty de la ISO) no hay truecolor: al
  arrancar se programan los 16 slots con OSC P y se restauran con OSC R al salir,
  también ante un pánico o Ctrl+C.
- Si se omite el paso **Equipo** se usa el hostname generado del modelo. Todas
  las elecciones por máquina (hostname, locale, zona, teclado, prettyHostName)
  van a `machine.toml` bajo `/persist/nixdots`; el instalador no modifica el
  repo (`system/lib/defaults.nix` se queda como está).
- El instalador **no crea la cuenta**: al reiniciar, el asistente de primer
  arranque pide usuario, contraseña y git, escribe la cuenta y el hash, y
  reconstruye el sistema.
