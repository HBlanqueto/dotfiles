# Per-machine facts that are not committed to the repo.
{
  config,
  lib,
  data,
  ...
}:

{
  options.system.machine = {
    encrypt = lib.mkOption {
      type = lib.types.bool;
      default = false;
      description = "Root (System HD) is a LUKS2 container.";
    };

    luksUuid = lib.mkOption {
      type = lib.types.str;
      default = "";
      description = "UUID of the LUKS partition (only meaningful with encrypt).";
    };

    espUuid = lib.mkOption {
      type = lib.types.str;
      default = "";
      description = ''
        UUID of the reused Windows ESP. When set, /boot mounts it by
        UUID (a reused ESP keeps its own label, so by-label/BOOT would
        be wrong); when empty, the freshly created ESP is used.
      '';
    };

    secureBoot = lib.mkOption {
      type = lib.types.bool;
      default = false;
      description = "Lanzaboote with own keys (Secure Boot).";
    };

    tpmUnlock = lib.mkOption {
      type = lib.types.bool;
      default = false;
      description = "TPM2 auto-unlock of the LUKS root (needs Secure Boot).";
    };
  };

  config = {
    system.machine = {
      encrypt = lib.mkDefault (data.machine.encrypt or false);
      luksUuid = lib.mkDefault (data.machine.luksUuid or "");
      espUuid = lib.mkDefault (data.machine.espUuid or "");
      secureBoot = lib.mkDefault (data.machine.secureBoot or false);
      tpmUnlock = lib.mkDefault (data.machine.tpmUnlock or false);
    };

    assertions = [
      {
        assertion = !config.system.machine.tpmUnlock || config.system.machine.secureBoot;
        message = "system.machine.tpmUnlock requiere system.machine.secureBoot";
      }
    ];
  };
}
