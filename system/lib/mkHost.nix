# Builds the nixosConfiguration for the generic profile.
{ inputs, theme }:
{
  system,
  modules ? [ ],
  requireImpure ? true,
  data,
}:
inputs.nixpkgs.lib.nixosSystem {
  inherit system;

  specialArgs = {
    inherit inputs theme;
    data = data // {
      hostName = data.machine.hostName or "system";
    };
  };

  modules = [
    ({ config, lib, ... }: {
      nixpkgs.hostPlatform = lib.mkIf (!config.hardware.facter.enable) (lib.mkDefault system);
    })
    ../core
    ../core/desktop
    ../users
    # Without impure eval the accounts can't be read; abort instead of treating the host as first boot (that would wipe the account).
    (_: {
      assertions = [
        {
          assertion = !requireImpure || data.impure;
          message = "system necesita evaluación impura para leer /persist/keel: usa rebuild-nixos sw o añade --impure";
        }
        {
          assertion = data.usersValid;
          message = data.usersError;
        }
      ];
    })
  ]
  ++ modules;
}
