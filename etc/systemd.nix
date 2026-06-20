{ config, pkgs, ... }:

{
    networking = {
        networkmanager.enable = true;
    };

    security.rtkit.enable = true;

    services = {
        acpid.enable = true;
        blueman.enable = true;
        bpftune.enable = true;
        dbus.enable = true;
        fstrim.enable = true;
        gnome.gnome-keyring.enable = true;
        gvfs.enable = true;
        libinput.enable = true;
        openssh.enable = true;
        printing.enable = true;
        qemuGuest.enable = true;
        spice-vdagentd.enable = true;
        upower.enable = true;

        pipewire = {
            enable = true;
            alsa = {
            enable = true;
            support32Bit = true;
        };
            jack.enable = true;
            pulse.enable = true;
        };
    };

    zramSwap = {
        enable = true;
        algorithm = "zstd";
        memoryPercent = 50;
    };

    systemd.oomd = {
        enable = true;
        enableRootSlice = true;
        enableUserSlices = true;
        enableSystemSlice = true;
        settings.OOM = {
        "DefaultMemoryPressureDurationSec" = "20s";
        };
    };

    nix = {
        settings.sandbox = false;
        optimise.automatic = true;

    gc = {
        automatic = true;
        dates = "weekly";
        options = "--delete-older-than 15d";
        persistent = true;
        };
    };
}