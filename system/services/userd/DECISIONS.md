# Decisiones de keel-userd

Registro de las decisiones que no se explican solas en el código: lo que se
cerró al implementar F0, F1a y lo que queda pendiente para fases futuras.

## F0 (fundaciones)

- **Nombres del bus.** Todas las constantes viven en
  `kits/userd-core/src/names.rs` (`BUS_NAME`, `ROOT_PATH`,
  `ERROR_NAMESPACE`, `POLKIT_PREFIX` e interfaces). Es el prefijo
  `io.github.HBlanqueto.Keel.Userd1` en F0; pasar a `org.nixos.*` es cambiar esas
  constantes. Los XML de `system/services/userd/interfaces/` y el derive
  `DbusError` necesitan literales por limitación de zbus; el test de
  conformidad (`bus.rs`) falla si dejan de coincidir con las constantes.
- **Superficie D-Bus de F0.** El XML de `Manager` solo declara lo implementado
  (`ListJobs`, `ListBackends`, `ConfigRejected`) y el de `Job` las propiedades,
  `Cancel` y `Finished`. El resto de interfaces del plan llega con su fase:
  escribir métodos sin implementación habría roto el test de conformidad.
- **Esquema KDL.** `version` es obligatorio y solo se acepta `1`; el resto de
  nodos y propiedades se rechaza si no se conoce, para que una errata no pase
  como campo ignorado. El "KDL v1" del plan es la versión del esquema: la
  sintaxis es KDL v2 (crate `kdl` 6). Los valores de `home.option` deben ser
  escalares (bool, número, string o null); arrays y objetos se rechazan.
- **Nombre de archivo.** `<usuario>.kdl` debe coincidir con el campo `user` del
  documento; si no, el watcher lo rechaza.
- **Jobs en F0.** `Scheduler::open` despacha los `Queued` al arrancar. Un actor
  sin handler deja el job en `Queued` (no lo descarta); `register` vuelve a
  despachar los `Queued` de ese `JobKind`, así que el orden de registro no
  pierde trabajo. `Running` y `Blocked` vuelven a `Queued` al arrancar; la
  re-evaluación de `Blocked` la hará el handler en F1 (vuelve a bloquear si la
  condición sigue). El test de integración `tests/resume.rs` comprueba que un
  job reanudado se ejecuta de verdad al registrar el handler.
- **Objetos `Job`.** `enqueue` registra `/Job/<id>` en el object server en el
  mismo momento de crear el job (el scheduler guarda la `Connection`). Los jobs
  que ya existían al arrancar se registran tras montar el `Manager`.
- **Job de prueba.** `keel-userd seed-job` vive detrás de la feature Cargo
  `test-hooks`. Solo el paquete `keel-userd-test`, que usa el nixosTest, la
  activa; el binario instalado no acepta `seed-job`. Se elimina cuando F1 cree
  jobs de verdad. Usa `JobKind::ModifyUser` como marcador.
- **Autorización.** `authz.rs` usa `zbus_polkit` y comprueba la acción
  `io.github.HBlanqueto.keel.userd.users.manage` en `Job.Cancel`. A partir de F0,
  cualquier método D-Bus que modifique estado debe pasar por `authz`; las
  acciones polkit ya están declaradas en `polkit/`.
- **Journal.** `put` se hace fuera del lock: se clona el registro, se suelta el
  lock, se persiste y luego se actualiza memoria. Los locks envenenados son
  `Error::LockPoisoned`, no se ignoran. Los errores de redb son variantes
  tipadas (`RedbDatabase`, `RedbTransaction`, `RedbTable`, `RedbStorage`,
  `RedbCommit`) y los de JSON, `Serialization`.
- **Transiciones.** Todas las escrituras de estado pasan por
  `Scheduler::transition`, serializado con un `tokio::sync::Mutex`. La sección
  lee, comprueba el estado esperado, persiste y actualiza memoria; las lecturas
  usan el `RwLock` y no esperan al disco. Así un `Cancel` que llega mientras el
  actor espera el semáforo no puede ser sobrescrito por `Running`, y el handler
  no se ejecuta.
- **Sondeo de home-manager.** Requiere `inputs.json` y el socket de
  `nix-daemon`; si falta cualquiera, queda no disponible con su motivo.
- **Servicio systemd.** `Type = "dbus"` con `BusName`, para que arranque cuando
  el nombre esté en el bus (se pide después de registrar las interfaces).
- **Store.** Escritura atómica (temporal en el mismo directorio, fsync del
  archivo, rename y fsync del directorio) con permisos 0600. El watcher inotify
  mantiene en memoria el último estado válido: si un KDL editado a mano no
  parsea, no reemplaza nada y se emite `Manager.ConfigRejected`. En el arranque
  los `ConfigRejected` se encolan y se emiten cuando el bus ya está conectado.
- **ListBackends.** Sondea los seis backends conocidos en el orden de prioridad.
  `userdb` se informa no disponible si `/etc/userdb` no se puede escribir
  (`access(W_OK)`); el daemon no crea ni asume escribible `/etc`. `homed` y
  `flatpak` miran el bus (propietario o activable); `nix-profile` el socket del
  daemon; `home-manager` la presencia de `inputs.json` (que F2 generará).
  `declared` siempre está disponible.
- **Módulo NixOS.** `keel.userd.enable = false` por defecto: ningún
  equipo cambia en F0. Con `enable = true`, el módulo añade `stateDir` a
  `environment.persistence."/persist".directories` (si `persistence.enable`,
  que por defecto es `true`) y crea la config, el servicio, la política D-Bus y
  la de polkit. F0 no aplica polkit: las acciones se declaran ahora para que F1
  solo tenga que comprobarlas.
- **Sin tocar crewd.** `system/services/crewd`, `home-init`,
  `system/users/default.nix` y `system/lib/data.nix` siguen siendo la fuente de
  verdad hasta F1.

## F1a (identidad)

- **Backends.** `userdb`, `homed` y `declared` coexisten; `declared` es de solo
  lectura. El orden por defecto es `userdb`, `homed`, `declared`; si `userdb`
  no está disponible, un usuario sin backend explícito cae en `homed`.
- **userdb.** Escribe registros JSON en `/etc/userdb` siguiendo la especificación
  de systemd-userdb: `<name>.user`, `<uid>.user`, `<name>.user-privileged`,
  `<name>.group` y `<name>:<group>.membership`. La contraseña se hashea con
  `mkpasswd -m yescrypt` y se guarda en `.user-privileged` (`/privileged/hashedPassword`).
- **homed.** Usa `org.freedesktop.home1.Manager.CreateHome` con
  `storage = "directory"`. El registro incluye `privileged.hashedPassword`,
  `secret.password` y `enforcePasswordPolicy = false` para evitar el rechazo de
  contraseñas sencillas en las VMs de prueba. Los UIDs de homed se asignan en el
  rango reservado `60001-60513` (`HOMED_UID_RANGE` en `bus/manager.rs`) porque
  systemd-homed espera usuarios "humanos" en ese rango.
- **PAM.** `pam_exec.so` con `expose_authtok` se sitúa antes que
  `pam_systemd_home.so` y `pam_unix.so` con control `sufficient`. Para usuarios
  userdb autentica directamente; para usuarios homed falla y deja que
  `pam_systemd_home.so` pregunte la contraseña y desbloquee el hogar. El helper
  lee la contraseña de stdin hasta EOF, no con `read_line`, porque
  `pam_exec` no envía salto de línea final.
- **Login en tests.** `su - <usuario>` desde root sigue pasando por la pila PAM,
  así que el test usa `expect` para enviar la contraseña en cada prompt
  `Password:` (incluido el segundo de `pam_systemd_home`).
- **Grupos extra.** Solo `video` se añade automáticamente; `networkmanager` se
  quitó porque homed valida la existencia de los grupos y `networkmanager` no
  siempre existe. Los administradores reciben `wheel`.
- **Persistencia.** `/etc/userdb`, `/var/lib/systemd/home` y
  `/var/lib/keel-userd` se añaden a `environment.persistence."/persist"`. Sin
  esto los usuarios userdb desaparecen tras el rollback de la raíz.
- **ArchiveUser.** El home se copia (no se renombra) a
  `stateDir/archive/<name>-<timestamp>` para evitar fallos por enlaces cruzados
  cuando `/home` y `/var/lib/keel-userd` están en diferentes filesystems. El UID
  liberado queda como *tombstone* en el registro y no se reutiliza.
- **SetPassword.** Rechaza usuarios declarados con `ReadOnly`; para userdb
  rehashea y escribe `.user-privileged`; para homed usa
  `ChangePasswordHome`.
- **nixosTest `userd-f1`.** Cubre alta/login de userdb y homed, sudo/polkit
  para admins, fallback a homed cuando `/etc/userdb` es de solo lectura,
  declared read-only, archive sin reutilización de UID y supervivencia tras
  reinicio.

## Fases futuras (no implementado)

- **F1 — crewd.** keel-userd sustituye a crewd: retirar el servicio, los TOML
  de `/persist/keel/users`, `hashedPasswordFile` y su aserción en
  `system/core/user.nix`, y dar un camino de migración para las cuentas
  existentes de la ThinkPad (leer los TOML y crear los usuarios con userdb o
  homed antes de apagar lo viejo).
- **Primer arranque.** Hoy `firstBoot` sale de `system/lib/data.nix` al evaluar.
  Pasará a decidirse en runtime (`HasNoUsers`), y el asistente de `welcome/`
  llamará a `CreateUser` en vez de escribir TOML y reconstruir.
- **F2 — home.** Pendiente decidir cómo convive con el flake del usuario en
  `~/.config/home-manager` (plantillas deck/raft y steward-home): los dos
  escriben el mismo perfil. No implementar F2 sin resolverlo.
- **F6 — capacidades.** Las unidades de NixOS no tienen `[Install]`, así que
  nada de `EnableUnitFiles`: usar `StartUnit`/`StopUnit` y reaplicar al arrancar
  el estado guardado en redb.
- **Build del home.** Preferir `nix build --impure --expr` sobre un flake
  generado (evita gestionar `flake.lock`).
