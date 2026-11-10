# Instalación de Keel

## Installer

`installer/` contains a text-mode installer (`nixdots-installer`). It is a Rust binary that *orchestrates* existing tools — `disko`, `nixos-facter`, `sbctl`, `nix`, `nixos-install`, `nmcli`, ... — and never reimplements partitioning.

There are two ways to run it:

* **Normal (precompiled, no compilation):** clone the repo and run the bootstrap, which downloads and runs the release binary for your architecture.

  ```shell
  nix-shell -p git --run "git clone --depth 1 https://github.com/HBlanqueto/nixdots"
  sudo ./nixdots/install.sh
  ```

  `install.sh` picks `x86_64`/`aarch64` from `uname -m`, downloads the binary from the [latest release](https://github.com/HBlanqueto/nixdots/releases/latest), verifies it against `SHA256SUMS` and runs it with `--install`, providing (via `nix shell`, pinned to `flake.lock`) the tools the live ISO lacks. The binaries are built and published by `.github/workflows/installer-release.yml` on every `v*` tag.

The installed system's TTY uses Terminus (`Lat2-Terminus16`, shipped with `kbd`) and the neptunia palette. Plymouth uses the built-in `spinner` theme with the Consoleet Darwin font.

* **Development (compiles from source):**

  ```shell
  sudo nix run .#installer -- --install   # real install from the NixOS ISO
  nix run .#installer                     # simulation: no disk, no root
  ```

The flake exposes it as `packages.{x86_64-linux,aarch64-linux}.{installer,default}` and `apps.<system>.{installer,default}`. See [installer/README.md](../../installer/README.md) for the full flow and options, and `system/disko.nix` for the disk layout it applies.

## Setup

> [!NOTE]
> I am not a Nix expert.

## Secure Boot (Lanzaboote)

> [!IMPORTANT]
> This configuration features a Zero-Touch Secure Boot setup.

Before booting the NixOS live USB, prepare your motherboard:

1. Enter your BIOS/UEFI settings.
2. Navigate to **Secure Boot**.
3. Select **Reset to Setup Mode** (or *Clear All Secure Boot Keys*).
4. Save and exit.

On the first boot after installation, Lanzaboote automatically generates the keys, bypasses `efivarfs` immutability to enroll them, and reboots the system. Because `/var/lib/auto-cryptenroll` and `/var/lib/sbctl` are persisted, no manual `sbctl` or `chattr` commands are ever required.

## Installation

The recommended path is the [installer](#installer):

```shell
nix-shell -p git --run "git clone --depth 1 https://github.com/HBlanqueto/nixdots"
sudo ./nixdots/install.sh
```

(development: `sudo nix run .#installer -- --install`). It partitions with
`system/disko.nix` (no manual partitioning needed), runs `nixos-facter`, writes
`machine.toml` and runs `nixos-install --flake <repo>#nixdots` for you. The
account is **not** created here: on the first reboot a Slint assistant asks for
the user, the password and git, writes the account files and the hash, and rebuilds.

To install manually, the flake provides the single `nixdots` configuration:

```shell
nixos-install --root /mnt --flake ~/nixdots#nixdots --impure --show-trace
```

## Disk encryption (LUKS + TPM)

`system/disko.nix` formats the root as a **LUKS2** container (`cryptroot`) holding the
btrfs subvolumes; only the ESP stays unencrypted (Lanzaboote must read the
kernel and initrd before unlocking). The installer asks for a dedicated LUKS
passphrase on the **Cifrado** screen (separate from the account password, which
does not exist yet) and early boots ask for it.

`system/core/luks-tpm.nix` enables Lanzaboote's **Measured Boot**
(`systemd-pcrlock`) on machines that ask for it. The installer enrolls a bridge
TPM2 token with no PCRs so Lanzaboote's `autoCryptenroll` can unlock on the
first boot and re-enroll, this time bound to the firmware, bootloader and
Secure Boot PCRs (`0 4 7`); `systemd-pcrlock` keeps the policy up to date
across Secure Boot and firmware changes, so boots unlock without typing. A
static `--tpm2-pcrs=7` would not do: PCR7 changes as soon as Lanzaboote enrolls
the Secure Boot keys. Until that first autoCryptenroll (the second boot) the
bridge token is not bound to any measurement. The `pcrlock` policy and
directory are persisted (`/var/lib/systemd/pcrlock.json`, `/var/lib/pcrlock.d`)
because the root is ephemeral. A machine without a TPM or without Secure Boot
(e.g. a VM) keeps asking for the passphrase.

> [!NOTE]
> Verify support with
> `/run/current-system/systemd/lib/systemd/systemd-pcrlock is-supported`
> (must print `yes`). If it does not, leave `"tpmUnlock": false` in `machine.toml`.

> [!NOTE]
> The password hash is written by the first-boot assistant to
> `/persist/secrets/<user>.hash` (`0600`, root-owned), creating the directory
> with `0711` so the unprivileged `nix` evaluation can still stat the file
> (`nixdots-rebuild` builds without sudo). It is never committed to the repo.
