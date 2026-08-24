{ config, pkgs, system, ... }:

let
    isAarch64 = system == "aarch64-linux";
in

{
    networking = {
        networkmanager.enable = true;
    };

    security = {

        sudo-rs = { 
            enable = true;
        };

        polkit = {
            enable = true;
        };

        rtkit = {
            enable = true;
        };

        pam = {
            services = {
                login = { 
                    enableGnomeKeyring = true;
                };
            };
        };
    };

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
        upower.enable = true;
        trezord.enable = true;

        # UTM
        # qemuGuest.enable = false;
        # spice-vdagentd.enable = false;

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