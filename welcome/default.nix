{ pkgs, mkRustTool, tablesEnv }:

let
  setup = mkRustTool {
    pname = "nixdots-setup";
    subdir = "welcome/cli";
    runtimeInputs = with pkgs; [
      git
      nix
      nixos-rebuild
      mkpasswd
      coreutils
    ];
    description = "First-boot assistant CLI for the Keel configuration";
    extra = tablesEnv pkgs;
  };
in
{
  inherit setup;

  assist-setup = mkRustTool {
    pname = "nixdots-setup-gui";
    subdir = "welcome/gui";
    description = "Graphical first-boot assistant for the Keel configuration (Slint)";
    runtimeInputs = [ setup ];
    extra = (tablesEnv pkgs) // {
      nativeBuildInputs = [
        pkgs.pkg-config
        pkgs.makeWrapper
      ];
      buildInputs = with pkgs; [
        fontconfig
        freetype
        libxkbcommon
        libglvnd
        wayland
        libx11
      ];
    };
  };
}
