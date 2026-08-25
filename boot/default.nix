{ config, pkgs, inputs, system, ... }:

let
    isAarch64 = system == "aarch64-linux";
in

{
    boot = {
        kernelPackages = if isAarch64 
                            then pkgs.linuxPackages_latest     
                        else pkgs.cachyosKernels.linuxPackages-cachyos-latest-lto-x86_64-v3;

        tmp = {
            cleanOnBoot = true;
        };

        loader = {
            systemd-boot.enable = true;
            efi.canTouchEfiVariables = true;
        };

        plymouth = {
            enable = true;
            theme = "mac-style";
            themePackages = [ pkgs.mac-style-plymouth ];
        };

        consoleLogLevel = 3;

        initrd = {
            verbose = false;
            systemd = {
            initrdBin = with pkgs; [ uutils-coreutils-noprefix btrfs-progs findutils util-linux ];

            # The BTRFS snapshot
            services = {
                rollback = {
                    wantedBy = [ "initrd.target" ];
                    before = [ "sysroot.mount" ];
                    unitConfig.DefaultDependencies = "no";
                    serviceConfig.Type = "oneshot";
                    script = builtins.readFile ./snapshot.sh;
                    };
                };
            };
        };

        kernelParams = [ "quiet" "splash" "rd.udev.log_level=3" "rd.systemd.show_status=auto" "microcode.amd_sha_check=off" ];

        kernel.sysctl = {
            "kernel.nmi_watchdog" = 0;
            "kernel.sched_cfs_bandwidth_slice_us" = 3000;
            "net.core.rmem_max" = 2500000;
            "vm.max_map_count" = 16777216;
            "vm.swappiness" = 100;
            "vm.page-cluster" = 0;
            "net.ipv4.tcp_mtu_probing" = 1;
            "net.ipv4.tcp_fastopen" = 3;
            "net.ipv4.tcp_fin_timeout" = 10;
            "net.core.default_qdisc" = "cake";
            "net.ipv4.tcp_congestion_control" = "bbr";
            "net.ipv4.tcp_slow_start_after_idle" = 0;
        };
    };
}