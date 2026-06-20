# 1. Atrapamos las variables globales inyectadas por el Flake
{ config, pkgs, inputs, lib, username, theme, ... }:

{
    home = {
        username = username;
        homeDirectory = "/home/${username}";

        sessionVariables = {
            BROWSER = "${pkgs.firefox}/bin/firefox";
            TERMINAL = "wezterm";
            EDITOR = "nvim";

            SDL_VIDEODRIVER = "wayland";
            QT_WAYLAND_DISABLE_WINDOWDECORATION = "1";
            QT_QPA_PLATFORM = "wayland;xcb";
            QT_QPA_PLATFORMTHEME = "gnome";
            CLUTTER_BACKEND = "wayland";
            NO_AT_BRIDGE = "1";
        };

        packages = with pkgs; [
            fastfetch
            eza
            mpc
            ffmpeg
            python3
            vscode
            nautilus
            telegram-desktop
        ];

        stateVersion = "26.05";
    }; # <--- ESTA LLAVE FALTABA PARA CERRAR 'home'

    programs = {
        brave = {
            enable = true;
        };
    
        git = {
            enable = true;
            settings = {
                user.email = "mc4w6wmkrv@privaterelay.appleid.com";
                user.name = "H. Blanqueto";
            };
        };

        yazi = {
            enable = true;
            enableFishIntegration = true;
            plugins = {
                inherit (pkgs.yaziPlugins) yatline;
            };
            initLua = builtins.readFile ./config/yazi/init.lua;
            settings = import ./config/yazi {};
        };

        ncmpcpp = {
            enable = true;
            package = pkgs.ncmpcpp.override { visualizerSupport = true; };
            settings = import ./config/ncmpcpp.nix;
        };

        bat = {
            enable = true;
            config = {
                pager = "never";
                style = "full";
                theme = "base16";
            };
        };

        wezterm = {
            enable = true;
            extraConfig = builtins.readFile ./config/wezterm.lua;
        };

        fish = {
            enable = true;
            interactiveShellInit = ''
                set -g fish_greeting 
                set -g fish_color_command --bold green""
            '';
            shellAliases = {
                delgen = "sudo nix-collect-garbage --delete-older-than 1d && sudo nix-store --gc && sudo nix-env --profile /nix/var/nix/profiles/system --delete-generations old";
                nix-update = "sudo nixos-rebuild switch";
                flake-update-rb = "sudo nixos-rebuild boot --flake .#nixos --impure";
                flake-update-sw = "sudo nixos-rebuild switch --flake .#nixos --impure";

                g = "git";
                c = "clear";
                ls = "eza --color=auto --icons";
                l = "ls -l";
                la = "ls -a";
                lla = "ls -la";
                lt = "ls --tree";
            };
        };

        starship = {
            enable = true;
            settings = import ./config/starship.nix;
        };

        home-manager = {
            enable = true;
        };
    };

    services = {
        mopidy = {
            enable = true;
            extensionPackages = with pkgs; [
                mopidy-local
                mopidy-mpd
            ];
            settings = import ./config/mopidy.nix {};
        };
    };

    fonts.fontconfig.enable = true;

    xdg = {
        enable = true;
        configFile = {
            "lua-module-color/theme.lua".text = ''
                return {
                    dbg = "#${theme.colors.dbg}",
                    lbg = "#${theme.colors.lbg}",
                    fg = "#${theme.colors.fg}",
                    bg = "#${theme.colors.bg}",
                    c0 = "#${theme.colors.c0}",
                    c1 = "#${theme.colors.c1}",
                    c2 = "#${theme.colors.c2}",
                    c3 = "#${theme.colors.c3}",
                    c4 = "#${theme.colors.c4}",
                    c5 = "#${theme.colors.c5}",
                    c6 = "#${theme.colors.c6}",
                    c7 = "#${theme.colors.c7}",
                    c8 = "#${theme.colors.c8}",
                    c9 = "#${theme.colors.c9}",
                    c10 = "#${theme.colors.c10}",
                    c11 = "#${theme.colors.c11}",
                    c12 = "#${theme.colors.c12}",
                    c13 = "#${theme.colors.c13}",
                    c14 = "#${theme.colors.c14}",
                    c15 = "#${theme.colors.c15}",
                }
            '';
        };

        userDirs = {
            enable = true;
            desktop = "${config.home.homeDirectory}/Escritorio";
            documents = "${config.home.homeDirectory}/Documentos";
            music = "${config.home.homeDirectory}/Música";
            pictures = "${config.home.homeDirectory}/Imágenes";
            videos = "${config.home.homeDirectory}/Videos";
            download = "${config.home.homeDirectory}/Descargas";
        };
    };

    nixpkgs.config = {
        allowUnfree = true;
    };
} # <--- ESTA LLAVE CIERRA TODO EL ARCHIVO