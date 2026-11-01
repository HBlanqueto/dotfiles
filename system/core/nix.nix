{
  lib,
  inputs,
  data,
  ...
}:

{
  nix = {
    settings = {
      experimental-features = [
        "nix-command"
        "flakes"
      ];

      allowed-users = [ "@users" ];
    };

    channel.enable = false;

    optimise.automatic = true;

    gc = {
      automatic = true;
      dates = "weekly";
      options = "--delete-older-than 14d";
    };
  };

  system.extraDependencies = lib.mapAttrsToList (_: i: i.outPath) (
    lib.filterAttrs (n: _: n != "self") inputs
  );

  nixpkgs = {
    config = {
      allowUnfree = true;
    };
    overlays = [
      (import ../overlays { inherit inputs; })
      inputs.brave-previews.overlays.default
      (_final: prev: {
        xorg = prev.xorg // {
          inherit (prev) libxcb;
        };
      })
    ];
  };

  documentation = {
    nixos.enable = false;
    man.enable = false;
  };

  system.stateVersion = data.settings.stateVersion;
}
