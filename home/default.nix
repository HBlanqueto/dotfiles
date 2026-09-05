{ config, pkgs, inputs, username, ... }:

{
    imports = [
        ./local/bin.nix
        ./local/style.nix
    ];

    home = {
        inherit username;
        homeDirectory = "/home/${username}";

        sessionVariables = {
            BROWSER = "${config.programs.brave.package}/bin/brave";
            TERMINAL = "wezterm";
            EDITOR = "nvim";
            SDL_VIDEODRIVER = "wayland";
            QT_QPA_PLATFORM = "wayland;xcb";
            CLUTTER_BACKEND = "wayland";
            NO_AT_BRIDGE = "1";
            NIXOS_OZONE_WL = "1";
        };

        file = {
            ".config/chime/chime.wav".source = ../thm/chime.wav;
        };
    };

    xdg = {
        enable = true;

        userDirs = {
            enable = true;
            createDirectories = true;

            projects = "${config.home.homeDirectory}/Proyectos";
            publicShare = "${config.home.homeDirectory}/Público";
            templates = "${config.home.homeDirectory}/Plantillas";
            desktop = "${config.home.homeDirectory}/Escritorio";
            documents = "${config.home.homeDirectory}/Documentos";
            music = "${config.home.homeDirectory}/Música";
            pictures = "${config.home.homeDirectory}/Imágenes";
            videos = "${config.home.homeDirectory}/Videos";
            download = "${config.home.homeDirectory}/Descargas";
        };
    };

    wayland = {
        windowManager = {
            hyprland = {
                enable = true;
        
                package = inputs.hyprland.packages.${pkgs.stdenv.hostPlatform.system}.hyprland;

                xwayland.enable = true;
                systemd.enable = false;
        
                plugins = [
                    inputs.hyprland-plugins.packages.${pkgs.stdenv.hostPlatform.system}.hyprbars
                    inputs.hyprland-plugins.packages.${pkgs.stdenv.hostPlatform.system}.borders-plus-plus
                ];

                extraConfig = ''
                    require('init')
                '';
            };
        };
    };
    
    nixpkgs = {
        config = {
            allowUnfree = true;

            permittedInsecurePackages = [
                "electron-39.8.10"
            ];
        };
    };
}