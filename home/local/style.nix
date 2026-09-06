{ pkgs, theme, ... }:

{
    home = {
        pointerCursor = {
            name = "Adwaita";
            package = pkgs.adwaita-icon-theme;
            gtk.enable = false;
            x11.enable = true;
        };

        sessionVariables = {
            QT_WAYLAND_DISABLE_WINDOWDECORATION = "1";
            QT_QPA_PLATFORMTHEME = "xdgdesktopportal";
        };
    };

    # Keep GTK mutable so dark mode can toggle dynamically via D-Bus/gsettings.
    gtk = {
        enable = false;
    };

    xdg = {
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

    fonts = {
        fontconfig = {
            enable = true;
        };
    };
}