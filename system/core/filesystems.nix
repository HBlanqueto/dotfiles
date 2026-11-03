# Device-agnostic filesystems addressed by label.
{ config, lib, ... }:

let
  btrfsOptions = import ../lib/btrfs-mount-options.nix ++ [ "space_cache=v2" ];

  btrfs = {
    device = "/dev/disk/by-label/nixos";
    fsType = "btrfs";
  };

  subvol = name: extra: [ "subvol=${name}" ] ++ btrfsOptions ++ extra;

  m = config.keel.machine;
in
{
  fileSystems = {
    "/" = btrfs // {
      options = subvol "@rootfs" [ ];
    };
    "/nix" = btrfs // {
      options = subvol "@nix" [ ];
    };
    "/home" = btrfs // {
      options = subvol "@home" [
        "nodev"
        "nosuid"
      ];
    };
    "/persist" = btrfs // {
      neededForBoot = true;
      options = subvol "@persist" [
        "nodev"
        "nosuid"
      ];
    };
    "/var/log" = btrfs // {
      options = subvol "@log" [
        "nodev"
        "nosuid"
        "noexec"
      ];
    };

    "/boot" = {
      device = if m.espUuid != "" then "/dev/disk/by-uuid/${m.espUuid}" else "/dev/disk/by-label/BOOT";
      fsType = "vfat";
      options = [
        "fmask=0077"
        "dmask=0077"
      ];
    };
  };

  services.btrfs.autoScrub = {
    enable = true;
    interval = "monthly";
    fileSystems = [ "/" ];
  };

  boot.initrd.luks.devices.cryptroot = lib.mkIf m.encrypt {
    device = "/dev/disk/by-uuid/${m.luksUuid}";
    allowDiscards = true;
  };

  assertions = [
    {
      assertion = !m.encrypt || m.luksUuid != "";
      message = "keel.machine.encrypt está activo pero keel.machine.luksUuid está vacío";
    }
  ];
}
