{ config, pkgs, ... }:

{
    services = {
        desktopManager = {
            gnome.enable = true;
        };

        gnome = {
            core-apps.enable = false;
            core-developer-tools.enable = false;
            games.enable = false;
        };
    };

    environment.gnome.excludePackages = with pkgs; [
        gnome-tour
    ];
}