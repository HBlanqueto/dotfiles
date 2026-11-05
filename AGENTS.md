# AGENTS.md — nixdots

Guía para agentes (opencode, Claude…) que trabajan en este repo. Describe el
estado **actual**; si algo de aquí contradice el código, manda el código y
actualiza este archivo en el mismo cambio.

## Rol

Claude actúa como ingeniero de software senior con la carrera completa de
Sistemas, Informática y Creación y Desarrollo de Software. Se enfoca en la
creación de entornos humanos, toolkits, el desarrollo de dependencias y
librerías, y los sistemas Unix y Unix-like.

## Forma de trabajar

- Aplica los cambios directamente. No escribas tests nuevos ni pidas
  verificaciones salvo que se pida. Si eliminas o renombras una función, elimina
  o adapta sus tests existentes para que el workspace siga compilando.
- Commits pequeños y lógicos con conventional commits
  (`feat(installer): …`, `fix(console): …`, `refactor(common): …`).
- No cambies `stateVersion` (system/lib/defaults.nix, "26.05").
- No añadas dependencias pesadas (inputs, crates, servicios) sin justificarlo en
  el commit.
- Si algo es ambiguo, elige la opción más simple y anótalo en el resumen final.
- Al terminar, resume en 1–3 líneas qué cambiaste y qué queda pendiente.
- Idioma: mensajes de la interfaz, documentación y los pocos comentarios que
  se permiten, en español.

## Código mínimo (reglas duras, estilo ponytail)

Se cumplen al pie de la letra. Ganan el diff más corto y la respuesta más corta.

1. **Reutiliza antes de escribir.** Busca con `grep` si ya existe la función, el
   token o el módulo. Cada valor y cada regla viven en un solo sitio; nunca copies.
2. **Nada «por si acaso».** Prohibido añadir parámetros, features, traits,
   genéricos, opciones o ramas que hoy nadie usa. El código muerto se borra: no
   se comenta ni se tapa con `#[allow(dead_code)]`;
   `deadnix` y `statix` deben quedar limpios.
3. **Lo más directo.** Nada de wrappers ni abstracciones de un solo uso. Se
   extrae a la tercera repetición, no antes.
4. **Una función, una tarea; un módulo, un motivo de cambio.** Si un bloque
   necesita un comentario para entenderse, extráelo a una función con ese nombre.
5. **Los nombres sustituyen al comentario:** `dias_transcurridos`, no `d`.
   Booleanos con forma de pregunta: `focused`, `zoom_supported`.
6. **Comentarios: ninguno por defecto.** Solo se permiten:
   - `// SAFETY:` en cada `unsafe`;
   - una línea con el *porqué* de un invariante cuya rotura desestabiliza y que
     el código no puede expresar: orden de protocolo, rodeo de un bug externo
     (con enlace) o seguridad. Cita el ADR si existe.

   Prohibido: explicar qué hace el código, contar historia («antes…», «ahora…»),
   `TODO` sin issue, código comentado, cabeceras decorativas y `///` que repiten
   el nombre. Si borras docs en un crate con `#![warn(missing_docs)]`, quita el
   lint en el mismo commit.
7. **Diff mínimo.** Toca solo lo que pide el cambio: no reformatees ni
   renombres de paso. El formato y los lints los ponen las herramientas.
8. **Tests:** no se escriben nuevos salvo que se pidan (ver «Forma de
   trabajar»). Si borras código, borra también sus tests.
9. **Ahorra tokens.** Lee rangos, no ficheros enteros. No pegues en el chat el
   código que ya está en el diff. Cierra con 1–3 líneas: qué cambió y qué queda.
10. **Investiga antes de suponer.** Si la causa o la API no está clara,
   consulta en internet, en este orden:
   1. el código y la documentación oficial de la dependencia **en la versión
      fijada**: el código de nixpkgs en el rev de `flake.lock`, el manual de NixOS, la documentación de disko, lanzaboote, nixos-facter y home-manager, y docs.rs para ratatui y Slint;
   2. los issues, PR, changelogs y discusiones del proyecto upstream;
   3. foros como Discourse, Stack Overflow o Reddit, solo como pista y
      contrastados con 1 o 2.

   No adoptes una solución que no puedas apoyar en una fuente. Si la solución
   es un rodeo, pon el enlace en el commit o en el comentario de una línea de la
   regla 6. En el chat va la conclusión con el enlace, no la investigación.

### Así se aplica en dotfiles

```nix
# ✗ copiar las opciones en disko.nix y en filesystems.nix
mountOptions = [ "noatime" "compress=zstd" "space_cache=v2" ];
# ✓ una lista compartida, y cada uno añade solo lo suyo
btrfsOptions = import ../lib/btrfs-mount-options.nix ++ [ "space_cache=v2" ];
```

```nix
# ✗ un módulo que lee /persist directamente
users = builtins.fromTOML (builtins.readFile "/persist/keel/users/${u}.toml");
# ✓ solo system/lib/data.nix lee archivos; los demás reciben `data`
{ data, ... }: { … data.machine.hostName … }
```

```nix
# ✓ comentario que se queda (mkHost.nix): su rotura borra datos
# Sin evaluación impura no se pueden leer las cuentas; abortar en vez de
# tratar el host como primer arranque (que borraría la cuenta).
```

- Los colores solo se definen en `kits/neptunia`. Lo de `generated/` se
  regenera; nunca se edita a mano.
- Las diferencias de hardware van en `hardware-detect.nix` vía facter, nunca
  por nombre de host.

## Arquitectura (no volver atrás)

- **Una sola configuración**: `nixosConfigurations.nixdots`. No hay `hosts/`.
  La arquitectura sale de `builtins.currentSystem` y el hardware del informe de
  **nixos-facter**. Por eso la evaluación real exige `--impure`; `nixdots-rebuild`
  lo añade. La salida también se expone con el hostname de la máquina.
- Variantes puras solo para CI: `nixdots-x86_64-linux` y
  `nixdots-aarch64-linux` (no leen /persist). VMs de prueba del primer
  arranque: `firstboot-vm` y `firstboot-vm-aarch64` (`dev.sh vm`).
- **Datos fuera del repo**, en `/persist/nixdots/`:
  - `machine.toml` — el archivo del sistema. Lo escriben el instalador
    (hostName, encrypt, luksUuid, espUuid, secureBoot, tpmUnlock, extras) y el
    asistente (prettyHostName, defaultLocale, region, timeZone, timeZoneAuto,
    keyboardLayout/Variant, autoLoginUser); gana el último en escribir. La
    escritura fusiona sobre el TOML existente. `espUuid` es el UUID de la ESP de
    Windows reutilizada (vacío si se crea nueva); `/boot` se monta por ese UUID
    en ese caso, y por label `BOOT` en el resto.
  - `facter.json` — informe de hardware.
  - `/persist/keel/users/<usuario>.toml` — una cuenta por archivo, solo lo esencial: `version`,
    `username`, `fullName` y `admin`. El nombre del archivo debe coincidir con
    `username`.
  - `/persist/keel/users/<usuario>.prefs.toml` — semilla de un solo uso para el home:
    apariencia y accesibilidad (colorScheme, accentColor, avatar*, a11y) y git.
    La escribe el asistente (y `nixdots-userd`) con modo 0600 y dueño del
    usuario; `nixdots-home-init` copia git a `home.nix` y el resto a
    `~/.config/home-manager/prefs.toml`, y borra la semilla.
  - `user.json` — **respaldo transitorio**: si no hay usuarios en `/persist/keel/users/*`, `data.nix` lo
    lee como un único usuario `admin = true`.
    `nixdots-setup migrate` lo convierte a `/persist/keel/users/<usuario>.toml` (+ su
    `.prefs.toml` con git) + `machine.toml` y deja `user.json.bak`.
  - Durante la migración el sistema lee el `.json` hermano cuando falta el
    `.toml` (en `system/lib/data.nix` y en `kits/nixcore/src/store.rs`); retira ese
    respaldo cuando todas las máquinas hayan corrido `migrate-toml`.
  - `/persist/secrets/<usuario>.hash` — hash de la contraseña
    (`hashedPasswordFile`), en un directorio `0711` y el archivo `0600`, ambos
    de root. Nunca hashes ni secretos en el repo.
- `system/lib/data.nix` es el **único** punto que lee esos archivos (getEnv,
  pathExists, readFile). Los módulos consumen `data` por specialArgs.
- `system/lib/defaults.nix` solo tiene valores independientes de la máquina (stateVersion,
  zona, locale, teclado por defecto).
- Diferencias de hardware: `system/core/hardware-detect.nix` decide con
  `config.hardware.facter.detected/report` (bare-metal AMD → ucodenix y amdgpu
  en initrd; portátil → acpid/upower; QEMU/UTM → spice-vdagent, qemu-guest).
  No uses condicionales por nombre de host.
- Opciones propias bajo `options.nixdots.*` (`machine`, `console`,
  `firstBoot`…). `nixdots.machine` (machine.toml) es la única capa de cifrado y
  Secure Boot. El instalador inscribe un token TPM2 puente sin PCRs
  (`--tpm2-pcrs=`) para que lanzaboote (`measuredBoot.autoCryptenroll`, con
  `systemd-pcrlock`) pueda desbloquear y reinscribir en el primer arranque la
  política atada a las PCR 0 4 7, que se actualiza sola; hasta ese arranque el
  token puente no está atado a ninguna medida.
- Kernel: es el de nixpkgs por defecto (LTS). Para cambiarlo, `boot.kernelPackages`
  en un módulo (no hay `nixdots.kernel`).
- Impermanence: la raíz se restaura en cada arranque (Btrfs `@rootfs` desde
  `@rootfs-blank`). Todo estado que deba sobrevivir se declara en
  `environment.persistence."/persist"`.

## Estructura

```text
dotfiles/
├── .github/        README corto, docs/ (instalador y sistema) y workflows/
├── system/         configuración NixOS, usuarios, servicios, plantillas y disko
├── installer/      instalador TUI, install.sh, launcher de kmscon y logo
├── welcome/        asistente del primer arranque (CLI, GUI Slint) y su VM
├── kits/           lo compartido: nixcore, localekit, neptunia y helpers Nix
├── flake.nix  flake.lock  Cargo.toml  Cargo.lock  .cargo/
└── AGENTS.md  dev.sh  install.sh  LICENSE  .gitignore  .git-blame-ignore-revs
```

- `.github/`: README corto, documentación (instalador y sistema) y workflows.
- `system/`: todo lo que acaba en el sistema instalado (NixOS, cuentas,
  plantillas de home, servicios, paquetes, overlays y disko). No va aquí nada
  que solo exista en la ISO o en el asistente.
- `installer/`: solo lo que corre desde la ISO (TUI, `install.sh`, launcher de
  kmscon y logo).
- `welcome/`: solo el asistente del primer arranque (CLI, GUI Slint, diseño y
  VM de prueba).
- `kits/`: lo compartido entre al menos dos de los anteriores (nixcore,
  localekit, neptunia y helpers Nix).

Nada nuevo en la raíz salvo archivos del flake, del workspace de Cargo y de git.
Los grep de rutas viejas pueden dar falsos positivos (p. ej. `/persist/keel/users`):
nunca reescribas código solo para silenciarlos.

## Componentes

- Tres capas:
  - `system/core/` — sistema (hardware, boot, consola, nix, seguridad,
    filesystems/rollback/persistence, servicios, GNOME, fuentes, extras y
    first-boot). No referencia a ningún usuario concreto ni rutas de `/home`.
    Incluye el paquete `home-manager` (system/core/user.nix) para el home
    independiente.
  - `system/users/` — cuentas: `system/users/default.nix` lee `data.users` y genera
    `users.users.<nombre>` (`hashedPasswordFile` derivado, `wheel` solo si
    `admin`). Ya **no** cablea home-manager: el home es independiente.
  - `system/users/templates/` — `nixdots` (home completo: apps, IA, escritorio) y
    `minimal` (fish, starship, git y el tema). `system/users/templates/default.nix` los
    envuelve e inyecta `theme`; el flake los exporta como `homeModules` para el
    home independiente. Reciben `prefs` (apariencia/a11y desde `prefs.toml`)
    por `extraSpecialArgs`, y la identidad de git por las opciones
    `nixdots.git.{name,email}`. `stateVersion` solo se define en `home.nix`;
    `timeZoneAuto` sale de `osConfig` (con valor por defecto fuera de NixOS).
    El home no instala herramientas de administración (nixdots-rebuild,
    flake-update-*…): viven en el sistema (`system/core/user.nix`).
- `system/templates/home/` — esqueleto (`flake.nix` + `home.nix`) que el flake publica
  como `templates.home`: un flake de home-manager que importa
  `nixdots.homeModules.{nixdots,minimal}` (una línea para cambiar), fija
  `nixpkgs.follows = "nixdots/nixpkgs"` y `home-manager.follows = "nixdots/home"`,
  aplica el overlay `nixdots.overlays.brave` y lee `./prefs.toml` para pasar
  `prefs`. El marcador `SYSTEM` lo rellena `nixdots-home-init`. El home real vive
  en `~/.config/home-manager`; se aplica con `home-manager switch`. Si ese
  directorio es un repo git, `flake.nix`, `home.nix` y `prefs.toml` deben estar
  rastreados (ver `system/templates/home/README.md`).
- `system/core/snapshot.sh` (ayudante de rollback) vive junto a `system/core/rollback.nix`.
- Escritorio: **solo GNOME**. SomeWM y Quickshell se quitaron; no los
  reintroduzcas sin que se pida.
- Otto (fork `github:HBlanqueto/otto`, con `layers` y `smithay-otto`) salió del
  sistema y solo se conserva como referencia externa, fuera de este repo: no lo
  reintroduzcas sin que se pida.
- Consola: la TTY instalada usa **solo** la fuente XNU (kmscon con el motor
  `psf`, `font-name` = `xnu-console.psf`, tamaño 32 —al doble—, y `console.font`
  = `xnu-console.psf` en el kernel: initrd y consola de emergencia). Solo tiene
  256 glifos Latin-1, así que en la TTY el prompt de fish (starship) y `eza` van
  sin iconos Nerd Font, y los caracteres fuera de Latin-1 salen como «?». El
  lanzador del instalador sí abre kmscon con JetBrainsMono Nerd Font Mono 14 y la
  paleta neptunia. Plymouth usa el tema integrado `spinner` con la fuente
  Consoleet Darwin (OTF, `pkgs.consoleet-darwin`).
- Workspace Rust (`Cargo.toml` raíz); los crates viven en `installer/`, `welcome/`, `system/services/` y `kits/`:
  - `kits/nixcore` — settings, /persist/keel/users/<usuario>.toml, machine.toml, hash,
    ejecución de comandos.
  - `kits/localekit` — locales, zonas y teclados embebidos al compilar.
  - `installer` — TUI ratatui (`nixdots-installer`). Solo instala el sistema;
    pantallas: Inicio · Licencia · Red · Lenguaje · Teclado · Zona horaria ·
    Equipo · Disco · Cifrado · Resumen · Instalando · Listo. No pide usuario,
    contraseña ni Git, ni escribe `/persist/keel/users/*.toml` ni el hash (eso lo hace el
    asistente). La frase de LUKS es siempre propia. Sin soporte de ratón.
  - `welcome/cli` — CLI del primer arranque (`nixdots-setup apply`) y de
    migración (`nixdots-setup migrate`).
  - `welcome/gui` — asistente gráfico «Hola» en **Slint** (no GTK/libadwaita);
    su modo `--home-init` muestra «Preparando tu escritorio» al primer inicio.
  - `system/services/crewd` — servicio de sistema por D-Bus
    (`org.nixdots.Users1`, política propia en el bus): `ListUsers`,
    `CreateUser`, `DeleteUser(keepHome)`, `SetAdmin`, `SetFullName`,
    `SetPassword` y `DeletePrefsSeed` (solo para el propio usuario). Las
    mutaciones devuelven un id de trabajo y publican `Progress`, `Finished`,
    `Failed` y `Warning`; la reconstrucción corre en `spawn_blocking`. Exige
    polkit `org.nixdots.users.manage` (auth_admin, acción en
    las acciones de polkit del sistema). Valida nombres (sin control, sin `:`, longitud) y
    reconstruye la versión instalada (`/etc/nixdots/flake-ref`) con la misma
    protección de usuarios que `nixdots-rebuild` (`kits/nixcore::rebuild`);
    un código 4 de `nixos-rebuild` no revierte, solo avisa de las unidades en
    fallo. `NIXDOTS_ROOT` va vacío (`/persist`).
  - `system/services/home-init` — servicio de usuario: si falta
    `~/.config/home-manager`, lo crea fijando `nixdots` a
    `github:HBlanqueto/nixdots` en la rev de `/etc/nixdots/rev` (o `path:` si la
    rev no es un commit), rellena `SYSTEM` con la arquitectura real, copia la
    semilla de prefs y ejecuta `home-manager switch`. Si el home ya existe,
    ignora la semilla y pide a userd que la borre.
- El sistema publica en `/etc/nixdots/flake-ref` el source exacto del flake
  instalado y en `/etc/nixdots/rev` su commit (`dirty` si se construyó con
  cambios); ambos se retienen en el closure para que userd pueda reconstruir.
- El instalador orquesta herramientas existentes (disko, nixos-facter, sbctl,
  nixos-install…); nunca reimplementa particionado. Los binarios se publican en
  la release `latest` desde `.github/workflows/installer-release.yml`;
  `install.sh` los descarga.

## Tema

- Una sola carpeta, `kits/neptunia/`, con dos fuentes; nada de colores
  hardcodeados en Nix, Lua, Rust ni Slint:
  - `kits/neptunia/neptunia.nix` (vía `kits/neptunia/default.nix`) — paleta
    de **terminal** (kmscon, wezterm, instalador y yazi), con `filecolors.nix`,
    `vscode.nix` y `palette.nix`.
  - `kits/neptunia/{tokens,components}.json` → `generated/` — **interfaz
    gráfica** (Slint, CSS). El asistente importa `Theme` y `Metrics` de
    `generated/neptunia.slint`.
- Copias generadas y versionadas (check `neptunia-generated` que falla si se
  desincronizan):
  - `kits/neptunia/generated/*` ←
    `nix eval --raw -f kits/neptunia {slint,css}`
  - `kits/neptunia/generated/kmscon-palette` ← `nix build .#kmscon-palette`

## Estilo

- Nix: respeta el estilo del archivo que tocas; no reformatees archivos enteros
  en un cambio funcional. Herramientas en `nix develop`: nixfmt, statix,
  deadnix, nil.
- Rust: rustfmt y `cargo clippy -D warnings` limpios. Errores con
  `color_eyre`/`Result`, nunca `unwrap()` en rutas que dependan del sistema.
  Secretos con `Zeroizing` y por stdin o archivo temporal, nunca por argv.
  `Cargo.toml` tiene dos perfiles: `release` (solo `panic = "abort"`, el que usa
  Nix) y `dist` (hereda el anterior y añade `opt-level = "z"`, `lto = true`,
  `codegen-units = 1` y `strip = true`; es el de las releases de GitHub,
  `cargo build --profile dist`).
- devShell: uno solo (`nix develop`), con Rust, Slint, formateadores y el CLI
  `nixdots-setup` para las simulaciones.
- Atajos: `./dev.sh sim installer|setup [--here]`, `./dev.sh vm [arch]` y
  `./dev.sh test [--no-build]`.
