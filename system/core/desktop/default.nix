{ pkgs, ... }:

{
  imports = [
    ./gnome.nix
    ./malaysia.nix
  ];

  hardware.graphics.enable = true;

  environment.sessionVariables.NIXOS_OZONE_WL = "1";

  environment.systemPackages = with pkgs; [
    libnotify
    playerctl
    wl-clipboard

    desktop-file-utils
    wayland-utils
    libinput
  ];
}
