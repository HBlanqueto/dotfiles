{ config, pkgs, lib, inputs, hostName, username, ... }:

let
    theme = import ../thm { };
in

{
    imports = [
        ./systemd.nix
        ./uutils.nix
        ./environment.nix
        ./fonts.nix
        ../hardware-configuration.nix
        ../boot
    ];

    home-manager = {
        backupFileExtension = "backup";
        sharedModules = [ ];
        extraSpecialArgs = { inherit inputs theme hostName username; };
        users.${username} = import ../home;
    };

    console = {
        colors = with theme.colors; [ bg c1 c2 c3 c4 c5 c6 c7 ] ++ [ lbg c9 c10 c11 c12 c13 c14 c15 ];
        font = "Lat2-Terminus16";
        keyMap = "es";
    };

    time.timeZone = "America/Merida";

    i18n.defaultLocale = "es_MX.UTF-8";

    networking.hostName = hostName;

    programs.fish.enable = true;

    environment.persistence."/persist" = {
        hideMounts = true;
        directories = [
            "/var/log"
            "/var/lib/bluetooth"
            "/var/lib/nixos"
            "/etc/NetworkManager/system-connections"
            "/var/lib/AccountsService"
            "/etc/nixos"
        ];
        files = [
            "/etc/machine-id"
        ];
    };

    users = { 
        defaultUserShell = pkgs.fish;
        mutableUsers = false;
        users.${username} = {
            isNormalUser = true;
            description = "H. Blanqueto";
            extraGroups = [ "networkmanager" "wheel" ];
            createHome = true;
            hashedPassword = "$6$rvI2ZNaKpc69XPeZ$R6iSUJ3l7iYlFc6eJz4pue1cl51d0H0dBNYkJcTm5BddRohQkdCC7sHmS50UczcPKESV//lw0CpO071roxsB21";
        };
    };

    environment = {
        binsh = "${pkgs.dash}/bin/dash";
        systemPackages = with pkgs; [
            git 
            wget 
            curl 
            polkit_gnome 
            gsettings-desktop-schemas 
            libnotify 
            firefox
            wezterm
        ];
    };

    nixpkgs = {
        config = {
            allowUnfree = true;
        };
        overlays = [ inputs.nix-cachyos-kernel.overlays.pinned ];
    };

    nix = {
        settings = {
            experimental-features = [ "nix-command" "flakes" ];
            substituters = [ "https://attic.xuyh0120.win/lantian" ];
            trusted-public-keys = [ "lantian:EeAUQ+W+6r7EtwnmYjeVwx5kOGEBpjlBfPlzGlTNvHc="];
        };
    };

    documentation.nixos.enable = false;

    hardware.enableRedistributableFirmware = true;

    system.stateVersion = "26.05";
}