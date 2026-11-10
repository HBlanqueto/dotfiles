{ pkgs, mkRustTool }:

{
  users-daemon = mkRustTool {
    pname = "users-daemon";
    subdir = "system/services/users-daemon";
    description = "D-Bus user management service";
  };

  home-init = mkRustTool {
    pname = "home-init";
    subdir = "system/services/home-init";
    description = "First-login home setup";
    runtimeInputs = [
      pkgs.home-manager
      pkgs.systemd
      pkgs.git
      pkgs.coreutils
    ];
  };

  rebuild-nixos = pkgs.callPackage ./packages/rebuild-nixos.nix { };
}
