{ inputs, theme }:

{
  mkHost = import ./lib/mkHost.nix { inherit inputs theme; };

  genericModules = [
    ./core/facter.nix
    ./core/filesystems.nix
    ./core/rollback.nix
    inputs.ucodenix.nixosModules.default
    inputs.lanzaboote.nixosModules.lanzaboote
    ./core/luks-tpm.nix
  ];

  homeModules = import ./users/templates;
}
