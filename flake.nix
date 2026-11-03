{
  description = "Welcome ~/*. Watch your step, it vanishes on boot.";

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

      keelSystem = import ./system { inherit inputs theme; };
      inherit (keelSystem) mkHost genericModules;

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

      mkFirstBootVm = import ./welcome/vm.nix {
        inherit
          nixpkgs
          inputs
          theme
          data
          ;
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
          inherit pkgs theme tablesEnv;
          mkRustTool = mkRustTool system;
        }
      );

      welcomePackages = forAllSystems (
        { system, pkgs }:
        import ./welcome {
          inherit pkgs;
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

      keel = mkGeneric {
        system = if data.impure then builtins.currentSystem else "x86_64-linux";
        requireImpure = true;
        inherit data;
      };
    in
    {
      nixosConfigurations = {
        inherit keel;

        keel-x86_64-linux = mkGeneric {
          system = "x86_64-linux";
          requireImpure = false;
          data = emptyData;
        };
        keel-aarch64-linux = mkGeneric {
          system = "aarch64-linux";
          requireImpure = false;
          data = emptyData;
        };

        firstboot-vm = mkFirstBootVm {
          system = "x86_64-linux";
          hostName = "firstboot-vm";
        };
        firstboot-vm-aarch64 = mkFirstBootVm {
          system = "aarch64-linux";
          hostName = "firstboot-vm-aarch64";
        };
      }
      // nixpkgs.lib.optionalAttrs (data.machine ? hostName) {
        ${data.machine.hostName} = keel;
      };

      overlays.brave = inputs.brave-previews.overlays.default;

      inherit (keelSystem) homeModules;

      templates = {
        home = {
          path = ./system/templates/home;
          description = "Home independiente de Keel (home-manager)";
        };
      };

      checks = forAllSystems (
        { pkgs, ... }:
        let
          palette = pkgs.callPackage ./installer/nix/kmscon-palette.nix { inherit theme; };
        in
        {
          neptunia-generated = pkgs.runCommand "check-neptunia-generated" { } ''
            if ! ${pkgs.diffutils}/bin/diff -u ${./kits/neptunia/generated/kmscon-palette} ${palette}; then
                echo "kits/neptunia/generated/kmscon-palette está desactualizado. Regéneralo con:" >&2
                echo "  cp \$(nix build .#kmscon-palette --print-out-paths) kits/neptunia/generated/kmscon-palette" >&2
                exit 1
            fi
            ${pkgs.coreutils}/bin/touch $out
          '';
        }
      );

      packages = forAllSystems (
        { system, ... }: {
          installer = installerPackages.${system}.installer;
          setup = welcomePackages.${system}.setup;
          crewd = systemPackages.${system}.crewd;
          home-init = systemPackages.${system}.home-init;
          launcher = installerPackages.${system}.launcher;
          default = installerPackages.${system}.launcher;

          kmscon-palette = installerPackages.${system}.kmscon-palette;

          nixdots-rebuild = systemPackages.${system}.nixdots-rebuild;
        }
      );

      apps = forAllSystems (
        { system, ... }: {
          installer = {
            type = "app";
            program = "${installerPackages.${system}.launcher}/bin/nixdots-installer";
          };
          default = {
            type = "app";
            program = "${installerPackages.${system}.launcher}/bin/nixdots-installer";
          };
        }
      );

      formatter = forAllSystems ({ pkgs, ... }: pkgs.nixfmt);

      devShells = forAllSystems (
        { system, pkgs, ... }:
        import ./kits/nix/devShells.nix {
          inherit pkgs tablesEnv;
          setup = welcomePackages.${system}.setup;
        }
      );
    };
}
