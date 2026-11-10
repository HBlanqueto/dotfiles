{ config, ... }:

{
  boot = {
    loader.efi.canTouchEfiVariables = true;

    consoleLogLevel = 3;

    initrd.verbose = false;

    initrd.systemd.emergencyAccess = config.system.machine.encrypt && !config.system.machine.tpmUnlock;

    plymouth = {
      enable = true;
      theme = "spinner";
    };

    kernelParams = [
      "quiet"
      "rd.udev.log_level=3"
      "rd.systemd.show_status=auto"
    ];

    kernel.sysctl = {
      "kernel.nmi_watchdog" = 0;
      "vm.swappiness" = 100;
      "vm.page-cluster" = 0;
      "net.core.default_qdisc" = "cake";
      "net.ipv4.tcp_congestion_control" = "bbr";
    };
  };
}
