{ pkgs, mkRustTool }:

{
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
    description = "First-boot text assistant for the Keel configuration";
  };
}
