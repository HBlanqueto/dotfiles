{ config, ... }:
{
  environment = {
    persistence."/persist" = {
      hideMounts = true;

      directories = [
        "/var/lib/bluetooth"
        "/var/lib/nixos"
        "/var/lib/systemd/timers"
        "/var/lib/systemd/backlight"
        "/var/lib/NetworkManager"
        "/var/lib/power-profiles-daemon"
        "/etc/NetworkManager/system-connections"
        "/var/lib/AccountsService"
        "/etc/nixos"
      ];

      files = [
        "/etc/machine-id"
        "/var/lib/systemd/credential.secret"
      ];
    };
  };

  system.activationScripts.systemCredentialSecret = {
    deps = [ "specialfs" ];
    text = ''
      # systemd-creds needs a regular 4112-byte file; let `systemd-creds setup` create it.
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

  system.activationScripts.persist-files.deps = [ "systemCredentialSecret" ];
}
