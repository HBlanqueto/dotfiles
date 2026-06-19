{
  description = "NixOS";

  inputs = { 
    unstable.url = "github:nixos/nixpkgs/nixos-unstable";
    home.url = "github:nix-community/home-manager";
    impermanence.url = "github:nix-community/impermanence";

    mac-style.url = "github:SergioRibera/s4rchiso-plymouth-theme";
    # brave-origin.url = "github:Daniel-42-z/brave-origin-flake"; # amd64 only

    nixpkgs.follows = "unstable";
  };

  outputs = { 
    self, 
    nixpkgs, 
    home,
    impermanence,
    ... 
  }@inputs:
  with nixpkgs.lib;
  let
    config = {
      allowUnfree = true;
    };

  filterNixFiles = k: v: v == "regular" && hasSuffix ".nix" k;

  importNixFiles = path:
    (lists.forEach (mapAttrsToList (name: _: path + ("/" + name))
      (filterAttrs filterNixFiles (builtins.readDir path)))) import;

  overlays = with inputs;
          [
            (final: _:
              let system = final.stdenv.hostPlatform.system;
              in {
              # Packages provided by flake inputs
              mac-style-plymouth = mac-style.packages.${system}.default;
              #brave-origin = brave-origin.${system}.default;
            }
          )
        ];
  in
  {
    nixosConfigurations = {

      utm-aarch64 = import ./sys {
        inherit config home inputs nixpkgs overlays ;
        system = "aarch64-linux";
      };
    };

    utm-aarch64 = self.nixosConfigurations.utm-aarch64.config.system.build.toplevel;
  };
}
