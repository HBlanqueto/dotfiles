{ inputs, ... }:

{
  imports = [
    inputs.impermanence.nixosModules.impermanence

    ./nix.nix
    ./persistence.nix
    ./machine.nix
    ./secure-boot.nix
    ./extras.nix
    ./hardware-detect.nix
    ./boot.nix
    ./user.nix
    ./first-boot.nix
    ./services.nix
    ./users-service.nix
    ./console.nix
    ./fonts.nix
    ./iphone-tethering.nix
  ];
}
