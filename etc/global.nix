{ pkgs, inputs, hostName, username, userdescription, hashedpassword, timeZone, defaultLocale, gitName, gitEmail, ... }:

let
    theme = import ../thm { };
in

{
    time = {
        timeZone = timeZone;
    };

    i18n = {
        defaultLocale = defaultLocale;
    };

    networking = {
        hostName = hostName;
    };

    users = { 
        defaultUserShell = pkgs.fish;
        mutableUsers = false;
        users.${username} = {
            isNormalUser = true;
            description = userdescription;
            extraGroups = [ "networkmanager" "wheel" ];
            createHome = true;
            hashedPassword = hashedpassword;
        };
    };

    home-manager = {
        backupFileExtension = "backup";
        sharedModules = [ ];
        extraSpecialArgs = { inherit inputs theme hostName username gitName gitEmail; };
        users.${username} = import ../home;
    };

    console = {
        colors = with theme.colors; [ bg c1 c2 c3 c4 c5 c6 c7 ] ++ [ lbg c9 c10 c11 c12 c13 c14 c15 ];
        font = "Lat2-Terminus16";
        keyMap = "es";
    };

    programs = {
        fish = {
            enable = true;
        };
    };

    environment = {
        binsh = "${pkgs.dash}/bin/dash";
        systemPackages = with pkgs; [
            sbctl
            git 
            wget 
            curl

            quickshell
            polkit_gnome 
            gsettings-desktop-schemas 
            libnotify

            wezterm
            firefox
        ];
    };
}