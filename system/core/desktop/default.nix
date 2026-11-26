{ pkgs, ... }:

{
  imports = [ ./gnome.nix ];

  hardware.graphics.enable = true;

  environment.sessionVariables.NIXOS_OZONE_WL = "1";

  # Claude Desktop downloads a generic Claude Code binary into the user profile.
  programs.nix-ld.enable = true;

  environment.systemPackages = with pkgs; [
    libnotify
    playerctl
    wl-clipboard

    desktop-file-utils
    wayland-utils
    libinput

    claude-desktop
  ];
}
