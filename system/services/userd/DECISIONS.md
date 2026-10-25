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

- **Backends.** `homed`, `userdb` y `declared` coexisten, pero el orden por
  defecto es `homed`, `declared`. `userdb` solo se usa si se pide
  explícitamente en `CreateUser`; no participa en el login. `declared` es de
  solo lectura.
- **userdb sin login.** En NixOS el módulo `nss-systemd` se consulta a través
  de `nsncd`, que corre como usuario `nscd`. `systemd-userdbd` solo entrega
  `privileged.hashedPassword` a clientes con UID 0 o al propio usuario, así que
  `nsncd` nunca ve el hash. El resultado es que `pam_unix` no puede autenticar
  usuarios userdb. Por eso `CreateUser` con `backend=userdb` y contraseña
  devuelve `InvalidConfig` ("userdb no admite login por contraseña en NixOS;
  usa homed"). No hay módulo PAM propio ni `pam_exec`: se descartó esa vía
  porque duplicaba autenticación y no resolvía el problema de fondo.
- **userdb como identidad.** Sigue escribiendo `<name>.user`, `<uid>.user`,
  `<name>.user-privileged`, `<name>.group` y `<name>:<group>.membership` para
  usos sin login. La contraseña se hashea con `mkpasswd -m yescrypt` y se
  guarda en `.user-privileged`, aunque no se use para PAM.
- **homed.** `CreateHome` recibe `privileged.hashedPassword` y
  `secret.password`; systemd-homed exige ambos para crear el home y propagar la
  contraseña. Además se genera una clave de recuperación `modhex64` como hace
  `homectl --recovery-key=yes`: su hash va en `privileged.recoveryKey`, el tipo
  en `recoveryKeyType` y la clave en claro en `secret.password` junto a la
  contraseña normal. El almacenamiento por defecto es `subvolume` y el test
  también usa `subvolume` sobre btrfs. Los UIDs se asignan en `60001-60513`
  (`HOMED_UID_RANGE` en `bus/manager.rs`).
- **Clave de recuperación.** `CreateUser` la entrega una sola vez con
  `Job.TakeRecoveryKey()`; no hay propiedad. El método exige que el UID del
  emisor coincida con el creador del job (guardado en memoria al encolarlo) y
  pasa por polkit `users.manage`. La clave vive en memoria del scheduler, no se
  escribe en redb, KDL ni logs, y se borra al tomarla o al purgar el job.
  Después de tomarla, de reiniciar el daemon o de purgar el job responde
  `NotFound`.
- **Custodia de la clave.** homed exige el texto plano de la clave de
  recuperación en cada `ChangePasswordHome` para recompilar todas las
  credenciales. Por eso, al crear un usuario homed, la clave se cifra con
  `systemd-creds encrypt --name=keel-userd-recovery-<name> --with-key=host`
  (clave del host, sin TPM obligatorio) y se guarda en
  `<stateDir>/recovery/<name>.cred`, con directorio `root:root 0700` y archivo
  `root:root 0600`. Nunca se escribe en texto plano en disco, redb, KDL ni
  logs. `TakeRecoveryKey` sigue existiendo para que el usuario apunte la clave,
  pero ya no es necesario para el uso normal.
- **SetPassword y AdminResetPassword.** `SetPassword(new, old)` descifra el
  `.cred`, pasa la contraseña antigua y la clave a `ChangePasswordHome` y deja
  que `Zeroizing` borre el texto plano. Autoriza al propio usuario con
  `users.password-own` o a un administrador con `users.manage`.
  `AdminResetPassword(new)` exige siempre `users.manage`, no pide la contraseña
  del usuario y usa la clave custodiada como único secreto antiguo.
  `ResetPassword(recovery_key, new)` se mantiene para el caso en que el usuario
  tenga la clave apuntada.
- **Sin `.cred`.** Si falta el archivo (cuenta creada antes de la custodia),
  `SetPassword` y `AdminResetPassword` devuelven un error claro que pide usar
  `ResetPassword` con la clave. No se intenta adivinar ni regenerar la clave.
- **Por qué no reduce la seguridad.** Con `storage = "subvolume"` o
  `"directory"`, homed no cifra el contenido del home: root ya puede leerlo y
  administrar la cuenta. Custodiar la clave solo evita perder la capacidad de
  cambiar la contraseña; no añade una vía nueva a datos que ya eran accesibles
  para root. La custodia se apoya en `credential.secret` de systemd, que vive
  en `/persist` y se crea antes del bind mount de impermanence. Si algún día se
  usa `storage = "luks"`, la custodia debe desactivarse: guardar la clave del
  home cifrado junto al host permitiría a root (o a quien tenga esa clave)
  saltarse LUKS. En ese caso el cambio de contraseña vuelve a depender de
  `ResetPassword` con la clave en manos del usuario.
- **Workaround de systemd 261.** `ChangePasswordHome` falla con `EOVERFLOW`
  («Value too large for defined data type») si `.identity*` y el contenedor del
  home no comparten dueño. Antes de cambiar la contraseña se alinean
  `.identity` y `.identity-blob` con el contenedor (`nobody:nogroup`). Es el
  workaround de systemd#38941 y no afecta al contenido del home.
- **PAM.** No se añade código PAM. Con `services.homed.enable`, NixOS pone
  `pam_systemd_home` en `login`, `polkit-1`, `sudo`, `su` y `systemd-user`;
  `gdm-password` lo hereda de `login` con `substack`. El test comprueba los
  seis ficheros.
- **Grupos extra.** Solo `video` se añade automáticamente; `networkmanager` se
  quitó porque homed valida que el grupo exista. Los administradores reciben
  `wheel` y `sudo`/`polkit` les piden contraseña (no se usa
  `wheelNeedsPassword = false`).
- **SetPassword en userdb.** Rehashea y reescribe `.user-privileged`. En homed
  usa `ChangePasswordHome`; la firma D-Bus es `SetPassword(ss)`
  (`password`, `old_password`). La contraseña antigua es necesaria para
  reautenticar el registro.
- **Persistencia.** `/var/lib/systemd/home`, `/var/lib/keel-userd` y
  `/var/lib/systemd/credential.secret` están en
  `environment.persistence."/persist"`. Los homes de homed viven en `/home`
  (subvolumen `@home`). El test reinicia la VM y comprueba que el home, la
  contraseña y el `.cred` siguen funcionando: `SetPassword` descifra la clave
  custodiada con la `credential.secret` persistida.
- **ArchiveUser.** El archivo vive en `/home/.keel-archive/<name>-<nanos>`,
  creado por el daemon con `root:root 0700`, dentro del mismo subvolumen que
  los homes. En userdb el home se renombra allí; en homed se mueve
  `/home/<name>.homedir` (directorio o subvolumen) con `rename` antes de
  `UnregisterHome`. Si el `rename` de un subvolumen falla, se usa
  `btrfs subvolume snapshot` + `delete`. Al terminar se borra el `.cred` del
  usuario. `keep_home=true` no mueve nada y deja
  esos datos en `/home/<name>.homedir` tras desregistrar la cuenta.
  systemd-homed puede redescubrir ese `.homedir` y volver a registrar la
  cuenta; es el precio de conservar los datos. El UID liberado queda como
  *tombstone* en el registro, sin reutilizarse.
- **nixosTest `userd-f1`.** Monta `/home` y `/persist` como subvolúmenes
  `@home` y `@persist` de un btrfs y usa `storage = "subvolume"`. Cubre
  usuarios homed, sin login userdb: alta, `.cred` cifrado y sin texto plano,
  `TakeRecoveryKey` una vez y `NotFound` la segunda, rechazo a un usuario no
  admin, reset con esa clave tanto por administrador como por el propio
  usuario, login por tty1 y `su`, sudo y pkexec con contraseña, `pamtester`
  sobre `gdm-password`, archive de homed con `keep_home` falso y verdadero en
  `/home/.keel-archive`, borrado del `.cred` al archivar, error claro si falta
  el `.cred`, `SetPassword` con la antigua tras reiniciar, `AdminResetPassword`
  sin la antigua, rechazo de `AdminResetPassword` a un no admin, declared de
  solo lectura y todo lo anterior tras reiniciar la VM. No usa
  `wheelNeedsPassword = false`.

## Fases futuras (no implementado)

- **F1 — crewd.** keel-userd sustituye a crewd: retirar el servicio, los TOML
  de `/persist/keel/users`, `hashedPasswordFile` y su aserción en
  `system/core/user.nix`, y dar un camino de migración para las cuentas
  existentes de la ThinkPad (leer los TOML y crear los usuarios con homed antes
  de apagar lo viejo). La migración de F1b cambiará el UID de la cuenta
  existente al rango de homed y hará `chown` del home.
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
