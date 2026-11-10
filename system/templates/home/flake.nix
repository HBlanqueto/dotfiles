{
  description = "My system home";

  inputs = {
    system.url = "github:HBlanqueto/dotfiles";

    nixpkgs.follows = "system/nixpkgs";

    home-manager.follows = "system/home";
  };

  outputs =
    {
      home-manager,
      nixpkgs,
      system,
      ...
    }:
    let
      arch = "SYSTEM";
    in
    {
      homeConfigurations.default = home-manager.lib.homeManagerConfiguration {
        pkgs = import nixpkgs {
          system = arch;

          config.allowUnfree = true;

          overlays = [
            system.overlays.brave
            (_final: prev: {
              xorg = prev.xorg // {
                inherit (prev) libxcb;
              };
            })
          ];
        };

        extraSpecialArgs = {
          inherit system;
          prefs =
            if builtins.pathExists ./prefs.toml then
              builtins.fromTOML (builtins.readFile ./prefs.toml)
            else
              { };
        };

        modules = [
          system.homeModules.deck
          ./home.nix
        ];
      };
    };
}
