{ inputs, pkgs, ... }:

let
  system = pkgs.stdenv.hostPlatform.system;
in
{
  imports = [ inputs.malaysia.nixosModules.default ];

  programs.malaysia.enable = true;

  environment.sessionVariables.LD_LIBRARY_PATH = [
    "${inputs.malaysia.packages.${system}.libadwaita-global-menu}/lib"
  ];

  environment.etc."firefox/policies/policies.json".text = builtins.toJSON {
    policies.Preferences = {
      "widget.gtk.global-menu.enabled" = true;
      "widget.gtk.global-menu.wayland.enabled" = true;
    };
  };
}
