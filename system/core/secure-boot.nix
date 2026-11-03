{
  config,
  lib,
  pkgs,
  ...
}:

let
  m = config.keel.machine;
in
{
  config = lib.mkMerge [
    (lib.mkIf m.secureBoot {
      boot.loader.systemd-boot.enable = lib.mkForce false;

      boot.lanzaboote = {
        enable = true;
        pkiBundle = "/var/lib/sbctl";

        configurationLimit = 4;

        autoGenerateKeys.enable = true;

        autoEnrollKeys = {
          enable = true;
          autoReboot = false;
        };
      };

      environment.persistence."/persist".directories = [
        "/var/lib/sbctl"
      ]
      ++ lib.optional (m.encrypt && m.tpmUnlock) "/var/lib/auto-cryptenroll";

      environment.systemPackages = with pkgs; [
        sbctl
        efibootmgr
      ];
    })

    (lib.mkIf (!m.secureBoot) {
      boot.loader.systemd-boot = {
        enable = lib.mkDefault true;
        configurationLimit = lib.mkDefault 5;
        editor = false;
      };
    })
  ];
}
