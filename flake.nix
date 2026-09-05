{
    description = "Welcome ~/*. Watch your step, it vanishes on boot.";

    inputs = {
        unstable.url = "github:nixos/nixpkgs/nixos-unstable";
        nixpkgs.follows = "unstable";

        home = {
            url = "github:nix-community/home-manager";
            inputs.nixpkgs.follows = "unstable";
        };

        hyprland.url = "github:hyprwm/Hyprland";
        hyprland-plugins = {
            url = "github:hyprwm/hyprland-plugins";
            inputs.hyprland.follows = "hyprland";
        };

        nix-cachyos-kernel.url = "github:xddxdd/nix-cachyos-kernel/release";
        impermanence.url = "github:nix-community/impermanence";
        ucodenix.url = "github:e-tho/ucodenix";

        brave-previews.url = "github:drishal/brave-browser-flake";
        mac-style.url = "github:SergioRibera/s4rchiso-plymouth-theme";
    };

    outputs = inputs: 
        let
            settings = import ./settings.nix;
        in 
        {
            nixosConfigurations = {
                "${settings.hostName}" = inputs.nixpkgs.lib.nixosSystem {
                    system = settings.system;

                    specialArgs = {
                        inherit inputs;
                        inherit (settings) system hostName timeZone defaultLocale username userdescription hashedpassword gitName gitEmail;
                    };

                    modules = [
                        ./etc
                        inputs.impermanence.nixosModules.impermanence
                        inputs.home.nixosModules.home-manager
                        inputs.ucodenix.nixosModules.default
                        {
                            nixpkgs.config.allowUnfree = true;
                            
                            nixpkgs.overlays = [
                                (final: prev: {
                                    mac-style-plymouth = inputs.mac-style.packages.${prev.stdenv.hostPlatform.system}.default;
                                })
                            ];
                        }
                    ];
                };
            };
        };
}