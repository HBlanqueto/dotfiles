{
  pkgs,
  mkRustTool,
  tablesEnv,
}:

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
      mkpasswd
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
}
