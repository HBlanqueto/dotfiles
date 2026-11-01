# First-boot setup, active only while `firstBoot` is set.
{
  config,
  lib,
  pkgs,
  inputs,
  data,
  ...
}:

let
  cfg = config.keel.assistSetup;

  guiCommand = pkgs.writeShellScript "assist-setup-gui" ''
    export XDG_RUNTIME_DIR=/run/user/0
    exec ${pkgs.dbus}/bin/dbus-run-session -- \
        ${inputs.self.packages.${pkgs.stdenv.hostPlatform.system}.assist-setup}/bin/nixdots-setup-gui
  '';

  firstBootSession = pkgs.writeShellScript "assist-setup" ''
    until ${pkgs.cage}/bin/cage -s -- ${cfg.command}; do
        ${pkgs.coreutils}/bin/sleep 1
    done
    ${pkgs.systemd}/bin/systemctl reboot
  '';
in

{
  options.keel.assistSetup.command = lib.mkOption {
    type = lib.types.str;
    default = "${guiCommand}";
    description = "Comando que se ejecuta en el primer arranque (el asistente gráfico).";
  };

  config = lib.mkIf data.firstBoot {
    users.allowNoPasswordLogin = true;

    services.displayManager.gdm.enable = lib.mkForce false;

    systemd.tmpfiles.rules = [ "d /run/user/0 0700 root root -" ];

    services.greetd = {
      enable = true;

      settings = {
        initial_session = {
          command = "${firstBootSession}";
          user = "root";
        };

        default_session = {
          command = "${pkgs.greetd}/bin/agreety --cmd ${pkgs.bash}/bin/bash";
        };
      };
    };
  };
}
