{ config, pkgs, ... }:

{
  security.rtkit.enable = true;

  networking = {
    networkmanager = {
      enable = true;
    };
  };

  services = {
    blueman.enable = true;
    printing.enable = true;
    upower.enable = true;
    openssh.enable = true;
    acpid.enable = true;
    gvfs.enable = true;
    gnome.gnome-keyring.enable = true;
    fstrim.enable = true;
    libinput.enable = true;
    dbus.enable = true;

    spice-vdagentd.enable = true;
    qemuGuest.enable = true;

    pipewire = {
      enable = true;

      alsa = {
        enable = true;
        support32Bit = true;
      };

      jack.enable = true;
      pulse.enable = true;
    };

      bpftune.enable = true;
  };
  
  nix = {
    gc = {
      automatic = true;
      persistent = true;
      dates = "weekly";
      options = "--delete-older-than 15d";
    };

    optimise.automatic = true;
    
    settings.sandbox = false;
  };

  zramSwap = {
    algorithm = "zstd";
    enable = true;
    memoryPercent = 50;
  };

  # Fedora enables these options by default. See the 10-oomd-* files here:
  # https://src.fedoraproject.org/rpms/systemd/tree/acb90c49c42276b06375a66c73673ac3510255
  systemd.oomd = {
    enable = true;
    enableRootSlice = true;
    enableUserSlices = true;
    enableSystemSlice = true;
    settings.OOM = {
      "DefaultMemoryPressureDurationSec" = "20s";
    };
  };
}
