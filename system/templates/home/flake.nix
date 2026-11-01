{
  description = "Mi home de Keel";

  inputs = {
    keel.url = "github:HBlanqueto/dotfiles";

    nixpkgs.follows = "keel/nixpkgs";

    home-manager.follows = "keel/home";
  };

  outputs =
    {
      home-manager,
      nixpkgs,
      keel,
      ...
    }:
    let
      system = "SYSTEM";
    in
    {
      homeConfigurations.default = home-manager.lib.homeManagerConfiguration {
        pkgs = import nixpkgs {
          inherit system;

          config.allowUnfree = true;

          overlays = [
            keel.overlays.brave
            (_final: prev: {
              xorg = prev.xorg // {
                inherit (prev) libxcb;
              };
            })
          ];
        };

        extraSpecialArgs = {
          inherit keel;
          prefs =
            if builtins.pathExists ./prefs.toml then
              builtins.fromTOML (builtins.readFile ./prefs.toml)
            else
              { };
        };

        modules = [
          keel.homeModules.deck
          ./home.nix
        ];
      };
    };
}
