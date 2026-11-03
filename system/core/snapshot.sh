#!/bin/sh

mkdir -p /btrfs_tmp
mount -t btrfs -o subvolid=5 /dev/disk/by-label/nixos /btrfs_tmp
trap 'umount /btrfs_tmp' EXIT

# Sin @rootfs-blank no hay con qué recrear la raíz: se arranca con la actual antes que sin /.
if [ ! -d /btrfs_tmp/@rootfs-blank ]; then
  echo "rollback: falta @rootfs-blank; se conserva @rootfs" >&2
  exit 1
fi

old=
if [ -e /btrfs_tmp/@rootfs ]; then
  mkdir -p /btrfs_tmp/old_roots
  old=$(mktemp -u -p /btrfs_tmp/old_roots "$(date "+%Y-%m-%d_%H-%M-%S").XXXX")
  mv /btrfs_tmp/@rootfs "$old"
fi

if ! btrfs subvolume snapshot /btrfs_tmp/@rootfs-blank /btrfs_tmp/@rootfs; then
  [ -n "$old" ] && mv "$old" /btrfs_tmp/@rootfs
  echo "rollback: falló el snapshot; se conserva @rootfs" >&2
  exit 1
fi

for snapshot in /btrfs_tmp/old_roots/*/; do
  [ -d "$snapshot" ] || continue
  [ "$(find "$snapshot" -maxdepth 0 -mtime +30)" ] || continue

  chattr -R -i "$snapshot" 2>/dev/null || true

  btrfs subvolume list -o "$snapshot" | awk '{print $NF}' | sort -r \
    | while read -r path; do
        btrfs subvolume delete "/btrfs_tmp/$path" 2>/dev/null || true
      done

  btrfs subvolume delete "$snapshot" 2>/dev/null \
    || echo "rollback: failed to prune $snapshot" >&2
done
