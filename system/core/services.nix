{
  config,
  pkgs,
  lib,
  ...
}:

{
  options.keel.ssh.enable = lib.mkEnableOption "the SSH server (keys only, persistent host keys)";

  config = {
    networking = {
      networkmanager.enable = true;
    };

    security = {
      sudo-rs = {
        enable = true;
        execWheelOnly = true;
      };

      rtkit = {
        enable = true;
      };
    };

    services = {
      bpftune.enable = true;

      openssh = lib.mkIf config.keel.ssh.enable {
        enable = true;
        settings = {
          PasswordAuthentication = false;
          KbdInteractiveAuthentication = false;
          PermitRootLogin = "no";
        };
        hostKeys = [
          {
            path = "/persist/etc/ssh/ssh_host_ed25519_key";
            type = "ed25519";
          }
        ];
      };

      avahi = lib.mkIf config.keel.features.epson {
        enable = true;
        nssmdns4 = true;
        publish.enable = false;
      };

      pipewire = {
        enable = true;
        wireplumber.enable = true;

        alsa = {
          enable = true;

          support32Bit = pkgs.stdenv.hostPlatform.isx86_64;
        };
        jack.enable = true;
        pulse.enable = true;
      };
    };

    systemd = {
      services.NetworkManager-wait-online.enable = false;

      oomd = {
        enable = true;
        enableRootSlice = true;
        enableUserSlices = true;
        enableSystemSlice = true;
        settings.OOM = {
          "DefaultMemoryPressureDurationSec" = "20s";
        };
      };
    };

    zramSwap = {
      enable = true;
      algorithm = "zstd";
      memoryPercent = 50;
    };
  };
}
