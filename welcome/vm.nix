{
  nixpkgs,
  inputs,
  theme,
  data,
}:
{ system, hostName }:
nixpkgs.lib.nixosSystem {
  inherit system;

  specialArgs = {
    inherit inputs theme;
    data = data // {
      inherit hostName;
    };
  };

  modules = [
    ../system/core/desktop
    ../system/core/nix.nix
    ../system/core/console.nix
    ../system/core/first-boot.nix
    ../system/core/user.nix
    ../system/users/default.nix
    { system.stateVersion = data.settings.stateVersion; }
    { users.users.root.password = "test"; }

    (nixpkgs.outPath + "/nixos/modules/virtualisation/qemu-vm.nix")

    ({ pkgs, ... }: {
      virtualisation = {
        memorySize = 4096;
        cores = 4;

        useBootLoader = true;
        useEFIBoot = true;
      };

      boot.loader.systemd-boot.enable = true;

      systemd.services.firstboot-vm-seed = {
        wantedBy = [ "multi-user.target" ];
        before = [ "greetd.service" ];
        serviceConfig.Type = "oneshot";
        path = with pkgs; [
          git
          coreutils
        ];
        script = ''
          if [ -e /persist/keel/repo ]; then exit 0; fi
          mkdir -p /persist/keel
          cp -a ${inputs.self} /persist/keel/repo
          chmod -R u+w /persist/keel/repo
          git -C /persist/keel/repo init -q
          git -C /persist/keel/repo add -A
          git -C /persist/keel/repo \
              -c user.name=seed -c user.email=seed@localhost \
              commit -qm seed
        '';
      };
    })
  ];
}
