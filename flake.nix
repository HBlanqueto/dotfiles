{
    description = "Welcome ~/*. Watch your step, it vanishes on boot.";

    inputs = {
        unstable.url = "github:nixos/nixpkgs/nixos-unstable";

        home.url = "github:nix-community/home-manager";
        parts.url = "github:hercules-ci/flake-parts";
        impermanence.url = "github:nix-community/impermanence";

        nix-cachyos-kernel.url = "github:xddxdd/nix-cachyos-kernel/release";

        brave-previews.url = "github:drishal/brave-browser-flake";

        mac-style.url = "github:SergioRibera/s4rchiso-plymouth-theme";

        nixpkgs.follows = "unstable";
    };

    outputs = inputs: inputs.parts.lib.mkFlake { inherit inputs; } {

        systems = [ "aarch64-linux" "x86_64-linux" ];

        flake = let
            settings = import ./settings.nix;
        in {
            nixosConfigurations = {
                
                "${settings.hostName}" = inputs.nixpkgs.lib.nixosSystem {
                    system = settings.system;

                    specialArgs = {
                        inherit inputs;
                        username = settings.username;
                        userdescription = settings.userdescription;
                        hashedpassword = settings.hashedpassword;

                        hostName = settings.hostName;
                        system = settings.system;
                    };

                    modules = [
                        ./etc
                        inputs.impermanence.nixosModules.impermanence
                        inputs.home.nixosModules.home-manager
                        inputs.brave-previews.nixosModules.default
                        {
                            nixpkgs.config.allowUnfree = true;
                            
                            networking.hostName = settings.hostName;

                            nixpkgs.overlays = [
                                (final: prev: {
                                    mac-style-plymouth = inputs.mac-style.packages.${prev.stdenv.hostPlatform.system}.default;
                                })
                            ];
                        }
                    ];
                };
            };
            
            "${settings.hostName}" = inputs.self.nixosConfigurations."${settings.hostName}".config.system.build.toplevel;
        };
    };
}