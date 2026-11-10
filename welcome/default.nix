{
  pkgs,
  mkRustTool,
  tablesEnv,
}:

let
  setup-nixos = mkRustTool {
    pname = "setup-nixos";
    subdir = "welcome/cli";
    runtimeInputs = with pkgs; [
      git
      nix
      nixos-rebuild
      mkpasswd
      coreutils
    ];
    description = "First-boot assistant CLI";
    extra = tablesEnv pkgs;
  };
in
{
  inherit setup-nixos;
}
