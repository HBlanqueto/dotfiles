{ pkgs, ... }:

{
  imports = [
    ./gnome.nix
    ./malaysia.nix
  ];

  hardware.graphics.enable = true;

  environment.systemPackages = with pkgs; [
    libnotify
    playerctl
    wl-clipboard

    desktop-file-utils
    wayland-utils
    libinput
  ];
}
