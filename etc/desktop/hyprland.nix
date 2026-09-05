{ pkgs, inputs, ... }:

{
    programs = { 
        hyprland = {
            enable = true;
            withUWSM = false;

            xwayland = { 
                enable = true;
            };

            package = inputs.hyprland.packages.${pkgs.stdenv.hostPlatform.system}.hyprland;
            portalPackage = inputs.hyprland.packages.${pkgs.stdenv.hostPlatform.system}.xdg-desktop-portal-hyprland;
        };
    };
}