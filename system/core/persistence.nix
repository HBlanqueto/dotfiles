{ config, pkgs, ... }:
{
  # Impermanence roots common to every machine. Persistent dirs that depend on
  # the hardware/roles (cups, sbctl, auto-cryptenroll, fwupd…) are
  # appended by system/core/extras.nix, secure-boot.nix, hardware-detect.nix
  # and luks-tpm.nix.
  environment = {
    persistence."/persist" = {
      hideMounts = true;

      directories = [
        "/var/lib/bluetooth"
        "/var/lib/nixos"
        # Con la raíz efímera esto se borra en cada arranque y los
        # timers Persistent=true (gc, optimise, scrub, fstrim) pierden
        # su última ejecución: si el equipo está apagado a su hora,
        # nunca corren.
        "/var/lib/systemd/timers"
        "/var/lib/systemd/backlight"
        # secret_key de MAC aleatoria y leases DHCP.
        "/var/lib/NetworkManager"
        # Perfiles de energía elegidos por el usuario.
        "/var/lib/power-profiles-daemon"
        "/etc/NetworkManager/system-connections"
        "/var/lib/AccountsService"
        "/etc/nixos"
      ];

      files = [
        "/etc/machine-id"
        # systemd-creds usa la clave del equipo; con la raíz efímera se
        # perdería en cada arranque y los servicios con credenciales
        # cifradas no arrancarían.
        #
        # systemd NO sigue enlaces simbólicos para esta clave, así que
        # debe ser un archivo real (bind mount). Impermanence
        # (rev 7b1d382) bind-montea los `files` con `method = "auto"`
        # solo si el origen ya existe; el servicio de abajo lo crea antes.
        "/var/lib/systemd/credential.secret"
      ];
    };
  };

  # systemd-creds necesita su clave antes de que el bind mount de
  # `/var/lib/systemd/credential.secret` esté activo. El script de activación
  # corre en initrd, donde `/persist` aún está en `/sysroot/persist` y todavía
  # no hay machine-id, así que se hace en un servicio de stage 2 ordenado antes
  # del bind mount de impermanence.
  systemd.services.keel-credential-secret = {
    description = "Crea la clave de systemd-creds en /persist";
    wantedBy = [ "local-fs.target" ];
    before = [
      "local-fs.target"
      "persist-persist-var-lib-systemd-credential.secret.service"
    ];
    after = [ "persist.mount" ];
    requires = [ "persist.mount" ];
    unitConfig.DefaultDependencies = false;
    path = [ pkgs.coreutils ];
    serviceConfig = {
      Type = "oneshot";
      RemainAfterExit = true;
    };
    script = ''
      # La activación de initrd puede haber dejado un symlink (systemd-creds
      # no los sigue); impermanence lo ignoraría por apuntar al origen.
      if [ -L /var/lib/systemd/credential.secret ]; then
          rm -f /var/lib/systemd/credential.secret
      fi
      if [ ! -e /persist/var/lib/systemd/credential.secret ]; then
          mkdir -p /persist/var/lib/systemd
          SYSTEMD_CREDENTIAL_SECRET=/persist/var/lib/systemd/credential.secret \
              ${config.systemd.package}/bin/systemd-creds setup
      fi
    '';
  };
}
