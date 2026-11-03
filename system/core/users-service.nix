{ pkgs, inputs, ... }:

let
  system = pkgs.stdenv.hostPlatform.system;

  userd = inputs.self.packages.${system}.crewd;
  homeInit = inputs.self.packages.${system}.home-init;

  polkitPolicy = pkgs.writeTextFile {
    name = "io.github.HBlanqueto.Keel.crew.manage.policy";
    destination = "/share/polkit-1/actions/io.github.HBlanqueto.Keel.crew.manage.policy";
    text = ''
      <?xml version="1.0" encoding="UTF-8"?>
      <!DOCTYPE policyconfig PUBLIC "-//freedesktop//DTD PolicyKit Policy Configuration 1.0//EN"
        "http://www.freedesktop.org/standards/PolicyKit/1/policyconfig.dtd">
      <policyconfig>
        <vendor>Keel</vendor>
        <action id="io.github.HBlanqueto.Keel.crew.manage">
          <description>Gestionar cuentas de Keel</description>
          <message>Se requiere autenticación de administrador para gestionar cuentas</message>
          <defaults>
            <allow_any>auth_admin</allow_any>
            <allow_inactive>auth_admin</allow_inactive>
            <allow_active>auth_admin</allow_active>
          </defaults>
        </action>
      </policyconfig>
    '';
  };

  dbusPolicy = pkgs.writeTextFile {
    name = "io.github.HBlanqueto.Keel.Crew1.conf";
    destination = "/share/dbus-1/system.d/io.github.HBlanqueto.Keel.Crew1.conf";
    text = ''
      <?xml version="1.0"?>
      <!DOCTYPE busconfig PUBLIC "-//freedesktop//DTD D-Bus Bus Configuration 1.0//EN"
        "http://www.freedesktop.org/standards/dbus/1.0/busconfig.dtd">
      <busconfig>
        <policy user="root">
          <allow own="io.github.HBlanqueto.Keel.Crew1"/>
        </policy>
        <policy context="default">
          <allow send_destination="io.github.HBlanqueto.Keel.Crew1"/>
          <allow receive_sender="io.github.HBlanqueto.Keel.Crew1"/>
        </policy>
      </busconfig>
    '';
  };
in
{
  environment = {
    etc."keel/source".text = inputs.self.outPath;
    etc."keel/release".text = ''
      commit = "${inputs.self.rev or "dirty"}"
      version = ""
      codename = ""
    '';

    systemPackages = [ polkitPolicy ];
  };

  system.extraDependencies = [ inputs.self.outPath ];

  security.polkit.enable = true;
  services.dbus.packages = [ dbusPolicy ];

  systemd.services.crewd = {
    description = "Keel user management service";
    wantedBy = [ "multi-user.target" ];
    after = [ "dbus.service" ];
    requires = [ "dbus.service" ];

    path = with pkgs; [
      nixos-rebuild
      nix
      git
      mkpasswd
      coreutils
    ];

    environment = {
      KEEL_ROOT = "";
      NIX_CONFIG = "experimental-features = nix-command flakes";
    };

    serviceConfig = {
      ExecStart = "${userd}/bin/nixdots-userd";
      Restart = "on-failure";
      RestartSec = 2;
    };
  };

  systemd.user.services.steward-home = {
    description = "Prepara el home de Keel";
    wantedBy = [ "default.target" ];
    after = [ "graphical-session.target" ];
    wants = [ "graphical-session.target" ];

    serviceConfig = {
      Type = "simple";
      ExecStart = "${homeInit}/bin/nixdots-home-init";
    };
  };
}
