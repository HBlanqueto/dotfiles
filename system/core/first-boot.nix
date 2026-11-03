{
  lib,
  pkgs,
  inputs,
  data,
  ...
}:

let
  setup = inputs.self.packages.${pkgs.stdenv.hostPlatform.system}.setup;

  firstBootSession = pkgs.writeShellScript "assist-setup" ''
    until ${setup}/bin/nixdots-setup; do
        ${pkgs.coreutils}/bin/stty sane
        echo "Vuelve a intentarlo."
    done
    ${pkgs.systemd}/bin/systemctl reboot
  '';
in

{
  config = lib.mkIf data.firstBoot {
    users.allowNoPasswordLogin = true;

    services.displayManager.gdm.enable = lib.mkForce false;

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
