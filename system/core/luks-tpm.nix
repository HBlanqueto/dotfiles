# TPM2 auto-unlock of the LUKS2 root.
{
  config,
  lib,
  pkgs,
  ...
}:

let
  m = config.keel.machine;

  device = config.boot.initrd.luks.devices."cryptroot".device;

  warning = "El desbloqueo con TPM no se completó; el disco está protegido solo por la entrada temporal. Ejecuta nixdots-rebuild sw o revisa auto-cryptenroll.";

  tpmBridgeLeft = pkgs.writeShellScript "lookout" ''
    ${pkgs.cryptsetup}/bin/cryptsetup \
        --external-tokens-path=${config.systemd.package}/lib/cryptsetup \
        luksDump ${lib.escapeShellArg device} | ${pkgs.gawk}/bin/awk '
            function flush() { if (inblock && !has) bad=1 }
            /^[[:space:]]*[0-9]+: systemd-tpm2/ { flush(); inblock=1; has=0; next }
            /^[[:space:]]*[0-9]+:/ { flush(); inblock=0 }
            inblock {
                if ($0 ~ /tpm2-pcrlock:[[:space:]]*yes/) has=1
                else if ($0 ~ /tpm2-hash-pcrs:[[:space:]]*[0-9]/) has=1
                else if ($0 ~ /tpm2-pubkey-pcrs:[[:space:]]*[0-9]/) has=1
            }
            END { flush(); exit bad ? 0 : 1 }
        '
  '';

  tpmWarn = pkgs.writeShellScript "lookout-notice" ''
    if [ -e /run/lookout/warning ]; then
        ${pkgs.libnotify}/bin/notify-send --urgency=critical --icon=dialog-warning \
            "Desbloqueo con TPM incompleto" ${lib.escapeShellArg warning}
    fi
  '';
in
{
  config = lib.mkIf (m.encrypt && m.tpmUnlock) {
    boot.lanzaboote = {
      measuredBoot = {
        enable = true;
        pcrs = [
          0
          4
          7
        ];

        autoCryptenroll = {
          enable = true;
          inherit device;
          autoReboot = false;
        };
      };
    };

    boot.initrd.luks.devices."cryptroot".crypttabExtraOpts = [
      "tpm2-device=auto"
    ];

    environment.persistence."/persist" = {
      directories = [ "/var/lib/pcrlock.d" ];
      files = [ "/var/lib/systemd/pcrlock.json" ];
    };

    systemd.services.lookout = {
      description = "Comprueba que auto-cryptenroll sustituyó el token TPM puente";
      after = [ "auto-cryptenroll.service" ];
      wants = [ "auto-cryptenroll.service" ];
      wantedBy = [ "multi-user.target" ];
      serviceConfig = {
        Type = "oneshot";
        RemainAfterExit = true;
        RuntimeDirectory = "lookout";
        RuntimeDirectoryMode = "0755";
      };
      script = ''
        if ${tpmBridgeLeft}; then
            printf '%s\n' ${lib.escapeShellArg warning} \
                | ${config.systemd.package}/bin/systemd-cat -t lookout -p err
            printf '%s\n' ${lib.escapeShellArg warning} > "$RUNTIME_DIRECTORY/warning"
        fi
      '';
    };

    systemd.user.services.lookout-notice = {
      description = "Avisa si el token TPM puente sigue activo";
      after = [ "graphical-session.target" ];
      wantedBy = [ "graphical-session.target" ];
      serviceConfig = {
        Type = "oneshot";
        RemainAfterExit = true;
        ExecStart = tpmWarn;
      };
    };
  };
}
