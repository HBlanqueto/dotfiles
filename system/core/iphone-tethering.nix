{
  config,
  lib,
  pkgs,
  ...
}:

{
  config = lib.mkIf config.system.features.iphoneTethering {
    services.usbmuxd.enable = true;

    boot.kernelModules = [ "ipheth" ];

    environment.systemPackages = with pkgs; [
      libimobiledevice
      ifuse
    ];
  };
}
