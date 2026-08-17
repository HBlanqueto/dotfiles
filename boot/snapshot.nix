{ pkgs, ... }:

''
    mkdir -p /btrfs_tmp
    mount /dev/nvme0n1p2 /btrfs_tmp

    if [ -e /btrfs_tmp/@rootfs ]; then
      mkdir -p /btrfs_tmp/old_roots
      timestamp=$(date "+%Y-%m-%d_%H:%M:%S")
      mv /btrfs_tmp/@rootfs /btrfs_tmp/old_roots/''${timestamp}
    fi

    if [ -d /btrfs_tmp/old_roots ]; then
      find /btrfs_tmp/old_roots/ -maxdepth 1 -mtime +30 -type d -exec btrfs subvolume delete {} \; 2>/dev/null || true
    fi

    btrfs subvolume snapshot /btrfs_tmp/rootfs-blank /btrfs_tmp/@rootfs
            
    umount /btrfs_tmp
''