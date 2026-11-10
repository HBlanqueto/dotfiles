# Sistema Keel

## Impermanence

Two details matter when partitioning by hand (the installer handles them):

- Append `neededForBoot = true;` to the `/persist` and `/var/log` file systems (NixOS already marks `/persist` and `/var/log` in `system/core/filesystems.nix`).
- For boot rollback, `system/core/snapshot.sh` must mount the Btrfs partition at the top level (`subvolid=5`, via the drive's UUID) so both `@rootfs` and `old_roots/` are visible; expired snapshots are pruned bottom-up (deepest nested subvolume first), because Btrfs refuses to delete a parent subvolume while children exist.

## Herramientas del sistema

### nixdots-rebuild

`nixdots-rebuild` es el wrapper seguro de `nixos-rebuild`; vive en
`system/packages/nixdots-rebuild.nix` y su lógica en `system/packages/nixdots-rebuild.sh`.
Siempre construye primero, compara los usuarios normales del sistema en marcha
con la configuración nueva para no perder cuentas y solo entonces activa. El
repositorio canónico es `~/dotfiles` (se puede cambiar con `KEEL_REPO`) y la
configuración es siempre `#keel`.

Modos:

- `sw` (o `switch`, por defecto): construye, comprueba y activa.
- `rb` (o `boot`): igual, pero deja la configuración para el siguiente arranque.
- `build`: solo construye; nunca activa.
- `update-sw`: ejecuta `nix flake update` y después `sw`.
- `update-rb`: ejecuta `nix flake update` y después `rb`.

Los alias de fish de `system/core/user.nix` son:

- `nix-update` → `nixdots-rebuild sw`
- `flake-update-rb` → `nixdots-rebuild update-rb`
- `flake-update-sw` → `nixdots-rebuild update-sw`
- `delgen` → `sudo nix-collect-garbage --delete-older-than 1d`

### Home Manager

El home es independiente del sistema. La plantilla `system/templates/home` (publicada
como `templates.home`) genera un flake de home-manager con `flake.nix` y
`home.nix`, importa `nixdots.homeModules.{nixdots,minimal}`, fija
`nixpkgs.follows = "nixdots/nixpkgs"` y `home-manager.follows = "nixdots/home"`,
aplica el overlay `nixdots.overlays.brave` y lee `./prefs.toml` para pasar
`prefs`. El marcador `SYSTEM` lo rellena `nixdots-home-init`.

El home real vive en `~/.config/home-manager` y se aplica con:

```shell
home-manager switch -b backup --flake ~/.config/home-manager#default
```

Si ese directorio es un repo git, `flake.nix`, `home.nix` y `prefs.toml` deben
estar rastreados (ver `system/templates/home/README.md`).

### keel.features

Las prestaciones opcionales se declaran en `system/core/extras.nix` bajo
`options.keel.features` y se activan por máquina desde `machine.toml`
(`"extras": { ... }`); sin datos quedan todas apagadas salvo
`iphoneTethering`, que está activa por defecto:

- `epson`: impresora/escáner Epson L3250.
- `trezor`: cartera Trezor (`trezord`).
- `chime`: sonido de arranque.
- `brightnessctl`: brillo y reglas udev para las teclas del panel.
- `remoteAccess`: clientes Remmina y RustDesk.
- `iphoneTethering`: tethering USB del iPhone.

## Acceso remoto: ThinkPad → Mac mini M4

La ThinkPad (NixOS, gestionada por este flake) se conecta al Mac mini M4
(sin Nix: nada de nix-darwin ni home-manager, todo manual) a través de la
red local.

- **Compartir pantalla de macOS (VNC)**: método principal y de emergencia,
  con **Remmina** como cliente en la ThinkPad.
- **RustDesk**: alternativa más cómoda para el día a día.

Lado NixOS: `system/core/extras.nix` (`nixdots.extras.remoteAccess`) sólo
instala los paquetes `remmina` y `rustdesk-flutter`. Se activa por máquina en
`machine.toml` (`"extras": { "remoteAccess": true }`).

> No guardes en el repo contraseñas de VNC ni el ID de RustDesk. Todo eso se
> queda en cada máquina.

### Mac mini (manual)

#### 1. Compartir pantalla (VNC)

1. **Ajustes del Sistema → General → Compartir**.
2. Activa **Pantalla remota** (*Screen Sharing*).
3. En el detalle (icono ⓘ), activa **Los visores VNC pueden controlar la
   pantalla con contraseña** y define una contraseña VNC. Añade los usuarios
   que puedan conectarse si usas "Sólo estos usuarios".
4. Deja **Inicio de sesión remoto** desactivado salvo que quieras SSH.

Anota la IP local del Mac en **Ajustes del Sistema → Red** (p. ej.
`192.168.x.y`); la usarás desde la ThinkPad.

#### 2. Energía: que el Mac no se duerma

En **Ajustes del Sistema → Energía** (en Mac mini sólo hay corriente):

- **Evitar el reposo automático cuando la pantalla está apagada**: activado.
- **Activar por acceso a la red** (*Wake for network access*): activado.
- **Reanudar tras un corte de corriente** (*Start up automatically after a
  power failure*): activado, para el acceso de emergencia.

Equivalentes por terminal (más fiables entre versiones de macOS):

```shell
sudo pmset -a sleep 0          # sin reposo del sistema
sudo pmset -a womp 1           # activar por acceso a la red
sudo pmset -a autorestart 1    # arrancar tras corte de corriente
pmset -g                       # verificar
```

#### 3. FileVault

Si FileVault está activo, **tras un reinicio no hay acceso remoto** hasta
desbloquear el disco en la pantalla. Para reinicios planificados usa:

```shell
sudo fdesetup authrestart
```

que reinicia guardando la autenticación para desbloquear el disco solo. Si
el Mac está a la espera de desbloqueo, sólo queda un reinicio local.

#### 4. RustDesk (opcional)

1. Instala RustDesk desde <https://rustdesk.com> (arrastra la app a
   Aplicaciones).
2. **Ajustes del Sistema → Privacidad y seguridad**: concede **Grabación de
   pantalla** y **Accesibilidad** a RustDesk; añade también **Monitoreo de
   entrada** si macOS lo pide.
3. Configura una contraseña permanente y, si quieres, restringe el acceso a
   una sola vez. El **ID de RustDesk no se guarda en el repo**: se copia a
   mano a la ThinkPad.

### ThinkPad (lado flake)

1. Reconstruye:

   ```shell
   sudo nixos-rebuild switch --flake ~/nixdots#nixdots
   ```

   (con `"remoteAccess": true` en `machine.toml` se instalan los paquetes
   `remmina` y `rustdesk-flutter`.)

2. En Remmina, **Nueva conexión**:
   - **Protocolo**: VNC.
   - **Servidor**: la IP local del Mac (`192.168.x.y`), visible en
     **Ajustes del Sistema → Red**.
   - **Contraseña**: la contraseña VNC del Mac.
   Guarda el perfil; a partir de ahí conectar es un doble clic.

3. RustDesk (opcional): abre RustDesk, pega el ID del Mac y su contraseña.

### Verificación rápida

```shell
ping <ip-del-mac>                   # alcanzable en la red local
remmina                             # doble clic en el perfil VNC
```
