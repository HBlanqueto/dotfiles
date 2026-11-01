#!/bin/sh

    mkdir -p /btrfs_tmp

    mount -t btrfs -o subvolid=5 /dev/disk/by-label/nixos /btrfs_tmp

    if [ -e /btrfs_tmp/@rootfs ]; then
      mkdir -p /btrfs_tmp/old_roots
      timestamp=$(date "+%Y-%m-%d_%H-%M-%S")
      
      mv -f /btrfs_tmp/@rootfs /btrfs_tmp/old_roots/${timestamp}
    fi

    if [ -d /btrfs_tmp/old_roots ]; then
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
    fi

    btrfs subvolume snapshot /btrfs_tmp/@rootfs-blank /btrfs_tmp/@rootfs

    umount /btrfs_tmp
