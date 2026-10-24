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
  };

  outputs =
    inputs@{ nixpkgs, ... }:
    let
      # Único punto de lectura de user.json/machine.json/facter.json: en
      # evaluación pura (nix flake check sin --impure) no ve /persist y
      # todo cae a los valores seguros, por eso la config real exige
      # impure (ver mkHost).
      data = import ./system/lib/data.nix { };

      # Variante sin datos externos: las salidas de CI no deben leer
      # /persist ni con --impure (no tienen user.json/machine.json/facter).
      emptyData = import ./system/lib/data.nix { impure = false; };

      # El tema (colores, filecolors, vscode) y el diseño se importan una
      # sola vez y bajan por specialArgs/extraSpecialArgs a los módulos,
      # y como argumento a los paquetes que los usan.
      theme = import ./kits/neptunia;

      keelSystem = import ./system { inherit inputs theme; };
      inherit (keelSystem) mkHost genericModules;

      # El perfil único detecta el hardware en tiempo de evaluación, así
      # que necesita impure (builtins.currentSystem + los .json en
      # /persist). En evaluación pura cae a x86_64-linux para que
      # `nix flake check` al menos liste la salida.
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
              # Las salidas sin cuenta (CI, o un equipo sin user.json)
              # satisfacen la assertion de NixOS "ningún usuario con
              # contraseña". Las de CI nunca se arrancan.
              users.allowNoPasswordLogin = true;
            };
        };

      # VM de prueba del primer arranque, en ambas arquitecturas (ver
      # welcome/vm.nix).
      mkFirstBootVm = import ./welcome/vm.nix {
        inherit
          nixpkgs
          inputs
          theme
          data
          ;
      };

      # The installer is built for both architectures the ISO is offered
      # on; it only orchestrates other tools.
      supportedSystems = [
        "x86_64-linux"
        "aarch64-linux"
      ];

      # Una salida por arquitectura: f recibe { system, pkgs }.
      forAllSystems =
        f:
        nixpkgs.lib.genAttrs supportedSystems (
          system:
          f {
            inherit system;
            pkgs = nixpkgs.legacyPackages.${system};
          }
        );

      # Data tables embedded at compile time with `include_str!` so the
      # binaries never shell out to `locale -a`, `localectl` or
      # `timedatectl` for their choices. Shared by the installer, the
      # setup GUI and the devShell.
      tablesEnv = pkgs: {
        KEEL_LOCALES = "${pkgs.glibcLocales}/share/i18n/SUPPORTED";
        # Source locale files (country_name / lang_name) for the
        # generated readable names; only the full glibc ships them.
        KEEL_LOCALE_DIR = "${pkgs.glibc}/share/i18n/locales";
        KEEL_XKB_BASE_LST = "${pkgs.xkeyboard_config}/share/X11/xkb/rules/base.lst";
        KEEL_ZONE1970_TAB = "${pkgs.tzdata}/share/zoneinfo/zone1970.tab";
      };

      # Constructor común de los binarios Rust (ver kits/nix/rustTool.nix).
      mkRustTool =
        system:
        import ./kits/nix/rustTool.nix {
          pkgs = nixpkgs.legacyPackages.${system};
          root = ./.;
          inherit supportedSystems;
        };

      # Instalador, lanzador de kmscon y paleta (ver installer/default.nix).
      installerPackages = forAllSystems (
        { system, pkgs }:
        import ./installer {
          inherit pkgs theme tablesEnv;
          mkRustTool = mkRustTool system;
        }
      );

      # Asistente de primer arranque (CLI + GUI) y su VM.
      welcomePackages = forAllSystems (
        { system, pkgs }:
        import ./welcome {
          inherit pkgs tablesEnv;
          mkRustTool = mkRustTool system;
        }
      );

      # Servicios y paquetes del sistema (ver system/packages.nix).
      systemPackages = forAllSystems (
        { system, pkgs }:
        import ./system/packages.nix {
          inherit pkgs;
          mkRustTool = mkRustTool system;
        }
      );

      # Perfil único: hardware detectado por facter + machine.json. La
      # forma impura resuelve la arquitectura con builtins.currentSystem.
      keel = mkGeneric {
        system = if data.impure then builtins.currentSystem else "x86_64-linux";
        requireImpure = true;
        inherit data;
      };
    in
    {
      nixosConfigurations = {
        inherit keel;

        # Variantes puras para CI (`nix flake check` / `nix build`
        # sin --impure): sin user.json/machine.json/facter.json, no
        # arrancan, solo comprueban que el árbol evalúa en ambas
        # arquitecturas.
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
      # El mismo sistema expuesto con el hostname del equipo, para que
      # `nixos-rebuild --flake ~/dotfiles` sin `#…` también funcione.
      // nixpkgs.lib.optionalAttrs (data.machine ? hostName) {
        ${data.machine.hostName} = keel;
      };

      # Overlay que el flake `home` importa para tener brave-origin-beta
      # (el sistema ya lo aplica en system/core/nix.nix).
      overlays.brave = inputs.brave-previews.overlays.default;

      # Módulos de home-manager reutilizables: `deck` (el home completo
      # de este repo) y `minimal` (fish, starship, git y el tema). El
      # template `home` genera un flake de usuario que los importa.
      inherit (keelSystem) homeModules;

      templates = {
        home = {
          path = ./system/templates/home;
          description = "Home independiente de Keel (home-manager)";
        };
      };

      # install.sh lee la copia versionada de la paleta sin pasar por
      # Nix; este check falla si dejó de coincidir con lo que genera
      # installer/nix/kmscon-palette.nix.
      checks = forAllSystems (
        { system, pkgs }:
        let
          palette = pkgs.callPackage ./installer/nix/kmscon-palette.nix { inherit theme; };
        in
        {
          # Las copias versionadas de generated/ deben coincidir con
          # lo que generan default.nix e installer/nix/kmscon-palette.nix.
          neptunia-generated = pkgs.runCommand "check-neptunia-generated" { } ''
            ok=1
            ${pkgs.diffutils}/bin/diff -u ${./kits/neptunia/generated/neptunia.slint} ${pkgs.writeText "g.slint" theme.slint} || ok=0
            ${pkgs.diffutils}/bin/diff -u ${./kits/neptunia/generated/neptunia.css} ${pkgs.writeText "g.css" theme.css} || ok=0
            ${pkgs.diffutils}/bin/diff -u ${./kits/neptunia/generated/Neptunia.qml} ${pkgs.writeText "g.qml" theme.qml} || ok=0
            ${pkgs.diffutils}/bin/diff -u ${./kits/neptunia/generated/kmscon-palette} ${palette} || ok=0
            if [ "$ok" = 0 ]; then
                echo "" >&2
                echo "kits/neptunia/generated está desactualizado. Regéneralo con:" >&2
                echo "  nix eval --raw -f kits/neptunia slint > kits/neptunia/generated/neptunia.slint" >&2
                echo "  nix eval --raw -f kits/neptunia css   > kits/neptunia/generated/neptunia.css" >&2
                echo "  nix eval --raw -f kits/neptunia qml   > kits/neptunia/generated/Neptunia.qml" >&2
                echo "  cp \$(nix build .#kmscon-palette --print-out-paths) kits/neptunia/generated/kmscon-palette" >&2
                exit 1
            fi
            ${pkgs.coreutils}/bin/touch $out
          '';

          # demo.slint must compile and render. NpButton is the smallest
          # component with a real size, so it exercises the whole file.
          slint-demo =
            pkgs.runCommand "check-slint-demo"
              {
                nativeBuildInputs = with pkgs; [
                  slint-viewer
                  xvfb-run
                ];
              }
              ''
                export HOME=$TMPDIR
                cd ${./kits/neptunia}
                xvfb-run -a slint-viewer demo.slint --component NpButton \
                  --screenshot $TMPDIR/demo.png --backend software
                test -s $TMPDIR/demo.png
                ${pkgs.coreutils}/bin/touch $out
              '';
        }
        # Arranca el daemon en una VM, consulta el bus y comprueba que un job
        # sobrevive a un reinicio del servicio. Solo x86_64: el runner de CI y
        # las máquinas del repo no emulan aarch64.
        // nixpkgs.lib.optionalAttrs (system == "x86_64-linux") {
          userd-f1 = import ./system/services/userd/tests/f1.nix {
            inherit pkgs inputs;
            daemon = systemPackages.${system}.keel-userd-test;
          };
        }
      );

      packages = forAllSystems (
        { system, pkgs }: {
          installer = installerPackages.${system}.installer;
          setup = welcomePackages.${system}.setup;
          assist-setup = welcomePackages.${system}.assist-setup;
          crewd = systemPackages.${system}.crewd;
          keel-userd = systemPackages.${system}.keel-userd;
          home-init = systemPackages.${system}.home-init;
          launcher = installerPackages.${system}.launcher;
          default = installerPackages.${system}.launcher;

          # The kmscon `--palette*` arguments, generated from the theme.
          # `install.sh` reads the committed copy at
          # kits/neptunia/generated/kmscon-palette; regenerate it with
          # `nix build .#kmscon-palette --print-out-paths`.
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

      # Development shell for the live WezTerm window: cargo + rehearse
      # on every save with watchexec.
      # `nix fmt`: el formateador oficial (RFC 166).
      formatter = forAllSystems ({ pkgs, ... }: pkgs.nixfmt);

      # Un único devShell para todo el repo (ver kits/nix/devShells.nix).
      devShells = forAllSystems (
        { system, pkgs, ... }:
        import ./kits/nix/devShells.nix {
          inherit pkgs tablesEnv;
          setup = welcomePackages.${system}.setup;
        }
      );
    };
}
