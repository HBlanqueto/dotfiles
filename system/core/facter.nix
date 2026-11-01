# Hardware detection from a nixos-facter report.
{ lib, data, ... }:

let
  readable = data.facterReport != null;
in
{
  hardware.facter = {
    reportPath = lib.mkIf readable data.facterReport;

    detected.dhcp.enable = lib.mkIf readable false;

    detected.fingerprint.enable = false;
  };
}
