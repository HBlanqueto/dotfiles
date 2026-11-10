# First-boot setup, active only while `firstBoot` is set.
{
  lib,
  pkgs,
  data,
  ...
}:

{
  config = lib.mkIf data.firstBoot {
    users.allowNoPasswordLogin = true;

    services.displayManager.gdm.enable = lib.mkForce false;

    services.greetd = {
      enable = true;

      settings = {
        initial_session = {
          command = "${pkgs.bash}/bin/bash -l";
          user = "root";
        };

        default_session = {
          command = "${pkgs.greetd}/bin/agreety --cmd ${pkgs.bash}/bin/bash";
        };
      };
    };
  };
}
