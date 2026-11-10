{
  description = "NixOS dotfiles.";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";

    home.url = "github:nix-community/home-manager";
    home.inputs.nixpkgs.follows = "nixpkgs";

    impermanence = {
      url = "github:nix-community/impermanence";
      inputs.nixpkgs.follows = "nixpkgs";
      inputs.home-manager.follows = "home";
    };

    ucodenix.url = "github:e-tho/ucodenix";

    lanzaboote.url = "github:nix-community/lanzaboote";
    lanzaboote.inputs.nixpkgs.follows = "nixpkgs";

    brave-previews.url = "github:drishal/brave-browser-flake";
    brave-previews.inputs.nixpkgs.follows = "nixpkgs";

    malaysia = {
      url = "github:HBlanqueto/malaysia";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs =
    inputs@{ nixpkgs, ... }:
    let
      data = import ./system/lib/data.nix { };

      emptyData = import ./system/lib/data.nix { impure = false; };

      theme = import ./kits/neptunia;

      systemConfig = import ./system { inherit inputs theme; };
      inherit (systemConfig) mkHost genericModules;

      mkGeneric =
        {
          system,
          requireImpure ? true,
          data,
        }:
        mkHost {
          inherit system requireImpure data;
          modules =
            genericModules
            ++ nixpkgs.lib.optional (!data.hasUsers) {
              users.allowNoPasswordLogin = true;
            };
        };

      supportedSystems = [
        "x86_64-linux"
        "aarch64-linux"
      ];

      forAllSystems =
        f:
        nixpkgs.lib.genAttrs supportedSystems (
          system:
          f {
            inherit system;
            pkgs = nixpkgs.legacyPackages.${system};
          }
        );

      tablesEnv = pkgs: {
        KEEL_LOCALES = "${pkgs.glibcLocales}/share/i18n/SUPPORTED";
        KEEL_LOCALE_DIR = "${pkgs.glibc}/share/i18n/locales";
        KEEL_XKB_BASE_LST = "${pkgs.xkeyboard_config}/share/X11/xkb/rules/base.lst";
        KEEL_ZONE1970_TAB = "${pkgs.tzdata}/share/zoneinfo/zone1970.tab";
      };

      mkRustTool =
        system:
        import ./kits/nix/rustTool.nix {
          pkgs = nixpkgs.legacyPackages.${system};
          root = ./.;
          inherit supportedSystems;
        };

      installerPackages = forAllSystems (
        { system, pkgs }:
        import ./installer {
          inherit pkgs tablesEnv;
          mkRustTool = mkRustTool system;
        }
      );

      systemPackages = forAllSystems (
        { system, pkgs }:
        import ./system/packages.nix {
          inherit pkgs;
          mkRustTool = mkRustTool system;
        }
      );

      nixosSystem = mkGeneric {
        system = if data.impure then builtins.currentSystem else "x86_64-linux";
        requireImpure = true;
        inherit data;
      };
    in
    {
      nixosConfigurations = {
        system = nixosSystem;

        system-x86_64-linux = mkGeneric {
          system = "x86_64-linux";
          requireImpure = false;
          data = emptyData;
        };
      }
      // nixpkgs.lib.optionalAttrs (data.machine ? hostName) {
        ${data.machine.hostName} = nixosSystem;
      };

      overlays.brave = inputs.brave-previews.overlays.default;

      inherit (systemConfig) homeModules;

      templates = {
        home = {
          path = ./system/templates/home;
          description = "Standalone system home-manager template";
        };
      };

      checks = forAllSystems (_: { });

      packages = forAllSystems (
        { system, ... }: {
          install-nixos = installerPackages.${system}.install-nixos;
          users-daemon = systemPackages.${system}.users-daemon;
          home-init = systemPackages.${system}.home-init;
          default = installerPackages.${system}.install-nixos;

          rebuild-nixos = systemPackages.${system}.rebuild-nixos;
        }
      );

      apps = forAllSystems (
        { system, ... }: {
          install-nixos = {
            type = "app";
            program = "${installerPackages.${system}.install-nixos}/bin/install-nixos";
          };
          default = {
            type = "app";
            program = "${installerPackages.${system}.install-nixos}/bin/install-nixos";
          };
        }
      );

      formatter = forAllSystems ({ pkgs, ... }: pkgs.nixfmt);

      devShells = forAllSystems (
        { system, pkgs, ... }:
        import ./kits/nix/devShells.nix {
          inherit pkgs tablesEnv;
        }
      );
    };
}
