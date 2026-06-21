{ config, pkgs, inputs, lib, username, theme, ... }:

{
    imports = [
        ./local
    ];

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
    };

    xdg = {
        enable = true;

        userDirs = {
            enable = true;
            desktop = "${config.home.homeDirectory}/Escritorio";
            documents = "${config.home.homeDirectory}/Documentos";
            music = "${config.home.homeDirectory}/Música";
            pictures = "${config.home.homeDirectory}/Imágenes";
            videos = "${config.home.homeDirectory}/Videos";
            download = "${config.home.homeDirectory}/Descargas";
        };

        configFile = {
            "lua-theme/theme.lua".text = ''
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
    };

    fonts.fontconfig.enable = true;

    nixpkgs.config = {
        allowUnfree = true;
    };
}