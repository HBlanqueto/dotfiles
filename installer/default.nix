{
  pkgs,
  mkRustTool,
  tablesEnv,
}:

let
  install-nixos = mkRustTool {
    pname = "install-nixos";
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
    description = "Text-mode NixOS installer";
    extra = tablesEnv pkgs;
  };
in
{
  inherit install-nixos;
}
