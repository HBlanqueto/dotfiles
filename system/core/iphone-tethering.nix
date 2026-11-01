{
  config,
  lib,
  pkgs,
  ...
}:

{
  config = lib.mkIf config.keel.features.iphoneTethering {
    services.usbmuxd.enable = true;

    boot.kernelModules = [ "ipheth" ];

    environment.systemPackages = with pkgs; [
      libimobiledevice
      ifuse
    ];
  };
}
