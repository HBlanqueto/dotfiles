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

  smbios = report.smbios.system or { };
  isThinkPadL14 =
    (smbios.manufacturer or "") == "LENOVO" && lib.hasPrefix "ThinkPad L14" (smbios.version or "");
in
{
  config = lib.mkIf config.hardware.facter.enable {
    services = {
      fwupd.enable = lib.mkIf isBaremetal true;

      spice-vdagentd.enable = lib.mkIf isQemu true;
      qemuGuest.enable = lib.mkIf isQemu true;

      ucodenix.enable = lib.mkIf (isBaremetal && hasAmdCpu) true;
    };

    # L14 quirks from nixos-hardware (lenovo/thinkpad/l14/amd).
    boot.kernelParams =
      lib.optional (isBaremetal && hasAmdCpu) "microcode.amd_sha_check=off"
      ++ lib.optional isThinkPadL14 "acpi_backlight=native"
      ++ lib.optional (isThinkPadL14 && hasAmdCpu) "iommu=soft";

    environment = {
      persistence."/persist".directories = lib.optional isBaremetal "/var/lib/fwupd";

      systemPackages = lib.optionals hasAmdGpu [
        pkgs.radeontop
        pkgs.libva-utils
      ];
    };
  };
}
