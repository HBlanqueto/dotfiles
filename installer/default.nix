{ pkgs, mkRustTool, theme, tablesEnv }:

let
  installer = mkRustTool {
    pname = "nixdots-installer";
    subdir = "installer";
    runtimeInputs = with pkgs; [
      disko
      btrfs-progs
      dosfstools
      util-linux
      cryptsetup
      systemd
      sbctl
      nixos-facter
      less
      git
      nix
    ];
    description = "Text-mode installer for the Keel NixOS configuration";
    extra = tablesEnv pkgs;
  };
in
{
  inherit installer;

  launcher = pkgs.callPackage ./nix/launcher.nix {
    inherit installer theme;
  };

  kmscon-palette = pkgs.callPackage ./nix/kmscon-palette.nix { inherit theme; };
}
