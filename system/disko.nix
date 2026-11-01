# Standalone disko disk layout.
{
  device ? "/dev/vda",
  luksPasswordFile ? null,
  encrypt ? "false",
  ...
}:
let
  doEncrypt = encrypt == "true";

  mountOptions = import ./lib/btrfs-mount-options.nix;

  btrfsContent = {
    type = "btrfs";
    extraArgs = [
      "-L"
      "nixos"
    ];

    subvolumes = {
      "@rootfs" = {
        mountpoint = "/";
        inherit mountOptions;
      };
      "@nix" = {
        mountpoint = "/nix";
        inherit mountOptions;
      };
      "@home" = {
        mountpoint = "/home";
        inherit mountOptions;
      };
      "@persist" = {
        mountpoint = "/persist";
        inherit mountOptions;
      };
      "@log" = {
        mountpoint = "/var/log";
        inherit mountOptions;
      };
    };

    postCreateHook = ''
      MNTPOINT=$(mktemp -d)
      mount "$device" "$MNTPOINT" -o subvol=/
      btrfs subvolume snapshot -r "$MNTPOINT/@rootfs" "$MNTPOINT/@rootfs-blank"
      umount "$MNTPOINT"
      rmdir "$MNTPOINT"
    '';
  };
in
{
  disko.devices = {
    disk.main = {
      type = "disk";
      inherit device;

      content = {
        type = "gpt";

        partitions = {
          ESP = {
            type = "EF00";
            size = "1G";

            content = {
              type = "filesystem";
              format = "vfat";
              mountpoint = "/boot";
              extraArgs = [
                "-n"
                "BOOT"
              ];
              mountOptions = [
                "fmask=0077"
                "dmask=0077"
              ];
            };
          };

          "System HD" = {
            size = "100%";
            label = "System HD";

            content =
              if doEncrypt then
                {
                  type = "luks";
                  name = "cryptroot";
                  passwordFile = luksPasswordFile;
                  settings.allowDiscards = true;
                  extraFormatArgs = [
                    "--type"
                    "luks2"
                  ];

                  content = btrfsContent;
                }
              else
                btrfsContent;
          };
        };
      };
    };
  };
}
