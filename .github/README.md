These files contain the foundation of my daily driver setup, tailored for both personal use and software development. Here you will find what you are probably looking for: my desktop configuration.

This NixOS system features the following core components:

* **Desktop Environment:** [GNOME](https://www.gnome.org/)
* **Window Manager / Compositor:** [Hyprland](https://hyprland.org/)
* **Shell:** [Fish](https://fishshell.com/)
* **Terminal:** [Wezterm](https://wezfurlong.org/wezterm/)
* **Editor:** [Neovim](https://neovim.io/)

## Environment

- [x] **Flakes:** Fully flake-based system configuration
- [x] **Home Manager:** Declarative user environment
- [x] **Impermanence:** Ephemeral root using Btrfs subvolumes
- [ ] **Lanzaboote & LUKS:** Secure Boot with Btrfs decryption
- [ ] **Agenix:** Secure secrets management

### Structure

An overview of the core configuration files and directories that make up this NixOS system:

```text
.
├── boot/
│   ├── default.nix
│   └── snapshot.sh
├── etc/
│   ├── desktop/
│   ├── default.nix
│   ├── fonts.nix
│   ├── global.nix
│   ├── systemd.nix
│   └── uutils.nix
├── home/
│   ├── config/
│   ├── local/
│   │   ├── bin.nix
│   │   └── style.nix
│   ├── thm/
│   └── default.nix
├── flake.lock
├── flake.nix
├── hardware-configuration.nix
└── settings.nix
```

* **`flake.nix`:** The main entry point for the system setup.
* **`settings.nix`:** Global variables (username, language, Git email).
* **`boot/`:** Startup settings, kernel, and Impermanence snapshot script.
* **`etc/`:** Core system environment (desktop, services, fonts, and system-wide tools).
* **`home/`:** User-specific configurations (XDG), including app configs, active programs (`bin.nix`), and theming (`style.nix`).

## Setup

> [!NOTE]
> I am not a Nix expert. 

> [!WARNING]
> This system may be considered unstable due to replacing `coreutils` with `uutils`. Disable this feature if necessary.

### Partitions

```shell
# Mount the Btrfs root to create subvolumes
mount /dev/[your_partition] /mnt

# Create subvolumes
btrfs su cr /mnt/@{rootfs,home,nix,persist,log}

# Create a blank snapshot for Impermanence
btrfs subvolume snapshot -r /mnt/@rootfs /mnt/@rootfs-blank
umount /mnt

# Mount the ephemeral root
mount -o subvol=@rootfs,noatime,compress=zstd,space_cache=v2 /dev/[your_partition] /mnt

# Create mount points
mkdir -p /mnt/{home,nix,persist,var/log,boot}

# Mount remaining subvolumes
mount -o subvol=@home,noatime,compress=zstd,space_cache=v2 /dev/[your_partition] /mnt/home
mount -o subvol=@nix,noatime,compress=zstd,space_cache=v2 /dev/[your_partition] /mnt/nix
mount -o subvol=@persist,noatime,compress=zstd,space_cache=v2 /dev/[your_partition] /mnt/persist
mount -o subvol=@log,noatime,compress=zstd,space_cache=v2 /dev/[your_partition] /mnt/var/log
```

### Generate hardware-configuration.nix

```shell
# Clone the repository
git clone https://github.com/HBlanqueto/dotfiles.git
cd dotfiles

# Generate hardware config and copy it to the repository
nixos-generate-config --root /mnt
cp /mnt/etc/nixos/hardware-configuration.nix ~/dotfiles
```
### Impermanence

#### hardware-configuration.nix

Open `~/dotfiles/hardware-configuration.nix` and append `neededForBoot = true;` to the `/persist` and `/var/log` file systems:

```nix
  fileSystems."/persist" = { 
    # ... your device and fsType ...
    options = [ "subvol=@persist" "noatime" "compress=zstd" "space_cache=v2" ];
    neededForBoot = true;
  };

  fileSystems."/var/log" = { 
    # ... your device and fsType ...
    options = [ "subvol=@log" "noatime" "compress=zstd" "space_cache=v2" ];
    neededForBoot = true;
  };
  ```

#### snapshot.sh

For the boot rollback to work, `boot/snapshot.sh` must mount your exact Btrfs partition. 

Replace the placeholder UUID with your drive's UUID (found in your generated `hardware-configuration.nix`):

```bash
#!/bin/sh

sleep 2
mkdir -p /btrfs_tmp

# Replace the UUID below with your actual partition UUID!
mount /dev/disk/by-uuid/YOUR-UUID-HERE /btrfs_tmp
```

### Installation

> [!NOTE]
> If you changed the hostname in `settings.nix`, make sure to replace `#nixos` with your new hostname in the command above.

  ```shell
nixos-install --root /mnt --flake '#nixos' --impure --show-trace
  ```

## Credits

Special thanks to these individuals whose work greatly inspired and guided the configuration of these files:

* [SergioRibera's dotfiles](https://github.com/SergioRibera/dotfiles)
* [chxp82q's nix-config](https://github.com/chxp82q/nix-config)
