{ config, pkgs, ... }:

{
  boot = {
    loader.efi.canTouchEfiVariables = true;

    consoleLogLevel = 3;

    initrd.verbose = false;

    initrd.systemd.emergencyAccess = config.keel.machine.encrypt && !config.keel.machine.tpmUnlock;

    plymouth = {
      enable = true;
      theme = "spinner";
      font = "${pkgs.consoleet-darwin}/share/fonts/opentype/consoleet_xnu.otf";
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
