{ config, pkgs, username, ... }:
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

        xserver = {
            enable = true;
            wacom.enable = false;
            excludePackages = [ pkgs.xterm ];
            
            xkb = {
                layout = "es";
                variant = "";
            };
        };

        displayManager = {
            gdm.enable = true;
        };
    

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

    systemd = {
        services = {

            chime = {
                enable = true;
                description = "Boot up";
                wants = ["sound.target"];
                after = [ "sound.target" "home-manager-${username}.service" ];
                wantedBy = ["multi-user.target"];

                serviceConfig = {
                    Type = "oneshot";
                    ExecStartPre = [
                    "-${pkgs.alsa-utils}/bin/amixer -c 1 sset Master 100% unmute"
                    "-${pkgs.alsa-utils}/bin/amixer -c 1 sset Speaker 100% unmute"
                ];

                    ExecStart = "${pkgs.alsa-utils}/bin/aplay -c 2 -D plughw:1,0 /home/${username}/chime/chime.wav";
                    RemainAfterExit = false;
                    SupplementaryGroups = "audio";
                };
            };
        };

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