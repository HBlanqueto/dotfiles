# Extra hardware wiring on top of the facter modules.
{
  config,
  lib,
  pkgs,
  ...
}:

let
  detected = config.hardware.facter.detected;
  report = config.hardware.facter.report;

  isBaremetal = detected.virtualisation.none.enable;
  isQemu = detected.virtualisation.qemu.enable;
  hasAmdGpu = detected.graphics.amd.enable;
  hasAmdCpu = lib.any (c: (c.vendor_name or "") == "AuthenticAMD") (report.hardware.cpu or [ ]);
in
{
  config = lib.mkIf config.hardware.facter.enable {
    services = {
      fwupd.enable = lib.mkIf isBaremetal true;

      spice-vdagentd.enable = lib.mkIf isQemu true;
      qemuGuest.enable = lib.mkIf isQemu true;

      ucodenix.enable = lib.mkIf (isBaremetal && hasAmdCpu) true;
    };

    boot.kernelParams = lib.mkIf (isBaremetal && hasAmdCpu) [ "microcode.amd_sha_check=off" ];

    environment = {
      persistence."/persist".directories = lib.optional isBaremetal "/var/lib/fwupd";

      systemPackages = lib.optional hasAmdGpu pkgs.radeontop;
    };
  };
}
