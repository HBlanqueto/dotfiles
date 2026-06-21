{
    description = "Welcome ~/*. Watch your step, it vanishes on boot.";

    inputs = {

        unstable.url = "github:nixos/nixpkgs/nixos-unstable";

        home.url = "github:nix-community/home-manager";
        parts.url = "github:hercules-ci/flake-parts";
        impermanence.url = "github:nix-community/impermanence";

        mac-style.url = "github:SergioRibera/s4rchiso-plymouth-theme";

        nixpkgs.follows = "unstable";
    };

    outputs = inputs: inputs.parts.lib.mkFlake { inherit inputs; } {

        systems = [ "aarch64-linux" "x86_64-linux" ];

        flake = {
            nixosConfigurations = {

                nixos = 
                    let
                        settings = import ./settings.nix;
                        currentSystem = builtins.currentSystem; 
                    in
                    inputs.nixpkgs.lib.nixosSystem {
                        system = currentSystem;

                        specialArgs = {
                            inherit inputs;
                            username = settings.username;
                            hostName = settings.hostName;
                        };

                        modules = [
                            ./etc
                            inputs.impermanence.nixosModules.impermanence
                            inputs.home.nixosModules.home-manager
                            {
                                nixpkgs.config.allowUnfree = true;
                                
                                networking.hostName = settings.hostName;

                                nixpkgs.overlays = [
                                    (final: prev: {
                                        mac-style-plymouth = inputs.mac-style.packages.${currentSystem}.default;
                                    })
                                ];
                            }
                        ];
                    };
                };
            nixos = inputs.self.nixosConfigurations.nixos.config.system.build.toplevel;
        };
    };
}