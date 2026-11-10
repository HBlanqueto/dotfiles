{
  pkgs,
  mkRustTool,
  tablesEnv,
}:

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
}
