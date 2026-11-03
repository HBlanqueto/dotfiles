{ pkgs, ... }:

{
  boot.initrd.systemd = {
    enable = true;

    initrdBin = with pkgs; [
      coreutils
      btrfs-progs
      findutils
      util-linux
      gawk
      e2fsprogs
    ];

    services.rollback = {
      wantedBy = [ "initrd.target" ];
      before = [ "sysroot.mount" ];
      requires = [ "dev-disk-by\\x2dlabel-nixos.device" ];
      after = [
        "dev-disk-by\\x2dlabel-nixos.device"
        "systemd-cryptsetup@cryptroot.service"
      ];
      unitConfig.DefaultDependencies = "no";
      serviceConfig.Type = "oneshot";
      script = builtins.readFile ./snapshot.sh;
    };
  };
}
