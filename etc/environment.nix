{ config, pkgs, ... }:

{
    services = {
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
    };
}