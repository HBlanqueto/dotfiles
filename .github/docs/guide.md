# Setup

## Drive Partitioning

```shell
# Temporarily mount the Btrfs disk root to carve out the subvolume structure
mount /dev/[your partition]

# Create decoupled subvolumes
btrfs su cr /mnt/@{rootfs,home,nix,persist,log}

# Capture a pristine snapshot of the empty root for the impermanence
btrfs subvolume snapshot -r /mnt/@rootfs /mnt/@rootfs-blank
umount /mnt

# Mount the actual ephemeral root (@rootfs)
mount -o subvol=@rootfs,noatime,compress=zstd,space_cache=v2 /dev/[your partition] /mnt

# Generate the hierarchical mount points required for the NixOS installation
mkdir -p /mnt/{home,nix,persist,var/log,boot}

# Mount persistent storage, Nix store cache, and system logs into the target hierarchy
mount -o subvol=@home,noatime,compress=zstd,space_cache=v2 /dev/[your partition] /mnt/home
mount -o subvol=@nix,noatime,compress=zstd,space_cache=v2 /dev/[your partition] /mnt/nix
mount -o subvol=@persist,noatime,compress=zstd,space_cache=v2 /dev/[your partition] /mnt/persist
mount -o subvol=@log,noatime,compress=zstd,space_cache=v2 /dev/[your partition] /mnt/var/log
```

## hardware-configuration.nix
**Impermanence** is implemented in this setup (wiping the root filesystem on every boot), NixOS needs to access the persistent storage and log paths *before* mounting the rest of the system. This requires manually modifying the generated configuration.

1. Generate the initial hardware layout profile from the mounted target system:

```shell
# Clone this dotfiles repository
git clone [https://github.com/HBlanqueto/dotfiles.git](https://github.com/HBlanqueto/dotfiles.git) --depth 1
cd dotfiles

# Generate the initial hardware layout configuration from the mounted target system
nixos-generate-config --root /mnt

# Copy the newly generated hardware profile into the declarative dotfiles setup
cp /mnt/etc/nixos/hardware-configuration.nix ~/dotfiles

```
Open `~/dotfiles/hardware-configuration.nix` and append `neededForBoot = true;` to both the `/persist` and `/var/log` file systems.

> **IMPORTANT**
> Without `neededForBoot = true;`, the machine will fail to boot because the impermanence modules and system logging services will attempt to load before their underlying Btrfs subvolumes are mounted.

Your file system declarations should match the following structure:

```nix
  fileSystems."/persist" =
    { device = "/dev/disk/by-uuid/5a387e47-22ae-4e1d-81da-b9ac913a48af";
      fsType = "btrfs";
      options = [ "subvol=@persist" "noatime" "compress=zstd" "space_cache=v2" ];
      neededForBoot = true;
    };

  fileSystems."/var/log" =
    { device = "/dev/disk/by-uuid/5a387e47-22ae-4e1d-81da-b9ac913a48af";
      fsType = "btrfs";
      options = [ "subvol=@log" "noatime" "compress=zstd" "space_cache=v2" ];
      neededForBoot = true;
    };
```

## Installation
```shell
nixos-install --root /mnt --flake '#nixos' --impure --show-trace
```