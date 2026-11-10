{ pkgs, mkRustTool }:

{
  crewd = mkRustTool {
    pname = "nixdots-userd";
    subdir = "system/services/crewd";
    description = "D-Bus user management service for Keel";
  };

  home-init = mkRustTool {
    pname = "nixdots-home-init";
    subdir = "system/services/home-init";
    description = "First-login home setup for Keel";
    runtimeInputs = [
      pkgs.home-manager
      pkgs.systemd
      pkgs.git
      pkgs.coreutils
    ];
  };

  nixdots-rebuild = pkgs.callPackage ./packages/nixdots-rebuild.nix { };
}
