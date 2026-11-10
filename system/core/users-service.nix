# Account management service (`users-daemon`) and first-login home setup.
{ pkgs, inputs, ... }:

let
  system = pkgs.stdenv.hostPlatform.system;

  userd = inputs.self.packages.${system}.users-daemon;
  homeInit = inputs.self.packages.${system}.home-init;

  homeInitSession = pkgs.writeShellScript "home-init-session" ''
    exec ${homeInit}/bin/home-init
  '';

  polkitPolicy = pkgs.writeTextFile {
    name = "io.github.HBlanqueto.System.users.manage.policy";
    destination = "/share/polkit-1/actions/io.github.HBlanqueto.System.users.manage.policy";
    text = ''
      <?xml version="1.0" encoding="UTF-8"?>
      <!DOCTYPE policyconfig PUBLIC "-//freedesktop//DTD PolicyKit Policy Configuration 1.0//EN"
        "http://www.freedesktop.org/standards/PolicyKit/1/policyconfig.dtd">
      <policyconfig>
        <vendor>System</vendor>
        <action id="io.github.HBlanqueto.System.users.manage">
          <description>Manage system accounts</description>
          <message>Administrator authentication is required to manage accounts</message>
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
    name = "io.github.HBlanqueto.System.Users1.conf";
    destination = "/share/dbus-1/system.d/io.github.HBlanqueto.System.Users1.conf";
    text = ''
      <?xml version="1.0"?>
      <!DOCTYPE busconfig PUBLIC "-//freedesktop//DTD D-Bus Bus Configuration 1.0//EN"
        "http://www.freedesktop.org/standards/dbus/1.0/busconfig.dtd">
      <busconfig>
        <policy user="root">
          <allow own="io.github.HBlanqueto.System.Users1"/>
        </policy>
        <policy context="default">
          <allow send_destination="io.github.HBlanqueto.System.Users1"/>
          <allow receive_sender="io.github.HBlanqueto.System.Users1"/>
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

  systemd.services.users-daemon = {
    description = "System user management service";
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
      ExecStart = "${userd}/bin/users-daemon";
      Restart = "on-failure";
      RestartSec = 2;
    };
  };

  systemd.user.services.home-init = {
    description = "System home setup";
    wantedBy = [ "default.target" ];
    after = [ "graphical-session.target" ];
    wants = [ "graphical-session.target" ];

    path = [
      homeInit
      pkgs.home-manager
      pkgs.systemd
      pkgs.git
      pkgs.coreutils
    ];

    serviceConfig = {
      Type = "simple";
      ExecStart = "${homeInitSession}";
    };
  };
}
