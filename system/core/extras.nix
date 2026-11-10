# Optional peripherals and roles, one switch per feature.
{
  config,
  lib,
  pkgs,
  data,
  ...
}:

let
  cfg = config.system.features;
  extraData = data.machine.extras or { };
in
{
  options.system.features = {
    epson = lib.mkEnableOption "the Epson L3250 printer/scanner";
    trezor = lib.mkEnableOption "the Trezor hardware wallet (trezord)";
    chime = lib.mkEnableOption "the boot chime";
    brightnessctl = lib.mkEnableOption "brightnessctl and its backlight hotkey rules";
    remoteAccess = lib.mkEnableOption "the Remmina and RustDesk remote access clients";
    iphoneTethering = lib.mkEnableOption "USB tethering from an iPhone" // {
      default = true;
    };
  };

  options.system.chimeCard = lib.mkOption {
    type = lib.types.int;
    default = 1;
    description = "ALSA card used by the boot chime.";
  };

  config = {
    system.features = {
      epson = lib.mkDefault (extraData.epson or false);
      trezor = lib.mkDefault (extraData.trezor or false);
      chime = lib.mkDefault (extraData.chime or false);
      brightnessctl = lib.mkDefault (extraData.brightnessctl or false);
      remoteAccess = lib.mkDefault (extraData.remoteAccess or false);
      iphoneTethering = lib.mkDefault (extraData.iphoneTethering or true);
    };

    system.chimeCard = lib.mkDefault (extraData.chimeCard or 1);

    services = {
      printing = lib.mkIf cfg.epson {
        enable = true;
        drivers = [ pkgs.epson-escpr ];
      };
      trezord.enable = cfg.trezor;
      udev.packages = lib.mkIf cfg.brightnessctl [ pkgs.brightnessctl ];
    };
    hardware.sane = lib.mkIf cfg.epson {
      enable = true;
      extraBackends = [ pkgs.epsonscan2 ];
    };

    systemd.services.chime = lib.mkIf cfg.chime {
      description = "Boot up";
      wants = [ "sound.target" ];
      after = [ "sound.target" ];
      wantedBy = [ "multi-user.target" ];
      restartIfChanged = false;

      serviceConfig = {
        Type = "oneshot";
        ExecStartPre = [
          "-${pkgs.alsa-utils}/bin/amixer -c ${toString config.system.chimeCard} sset Master 100% unmute"
          "-${pkgs.alsa-utils}/bin/amixer -c ${toString config.system.chimeCard} sset Speaker 100% unmute"
        ];
        ExecStart = "${pkgs.alsa-utils}/bin/aplay -c 2 -D plughw:${toString config.system.chimeCard},0 ${./chime.wav}";
        RemainAfterExit = false;
        SupplementaryGroups = "audio";
      };
    };

    environment.systemPackages =
      lib.optionals cfg.brightnessctl [ pkgs.brightnessctl ]
      ++ lib.optionals cfg.remoteAccess [
        pkgs.remmina
        pkgs.rustdesk-flutter
      ];

    environment.persistence."/persist".directories = lib.optionals cfg.epson [ "/var/lib/cups" ];
  };
}
