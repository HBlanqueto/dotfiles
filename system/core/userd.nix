# keel-userd (F0): esqueleto del servicio. Desactivado por defecto para que
# ningún equipo cambie hasta que F1 sustituya a crewd.
{
  config,
  lib,
  pkgs,
  inputs,
  ...
}:

let
  cfg = config.keel.userd;
  system = pkgs.stdenv.hostPlatform.system;

  dbusPolicy = pkgs.writeTextFile {
    name = "io.github.HBlanqueto.Keel.Userd1.conf";
    destination = "/share/dbus-1/system.d/io.github.HBlanqueto.Keel.Userd1.conf";
    text = ''
      <?xml version="1.0"?>
      <!DOCTYPE busconfig PUBLIC "-//freedesktop//DTD D-Bus Bus Configuration 1.0//EN"
        "http://www.freedesktop.org/standards/dbus/1.0/busconfig.dtd">
      <busconfig>
        <policy user="root">
          <allow own="io.github.HBlanqueto.Keel.Userd1"/>
        </policy>
        <policy context="default">
          <allow send_destination="io.github.HBlanqueto.Keel.Userd1"/>
          <allow receive_sender="io.github.HBlanqueto.Keel.Userd1"/>
        </policy>
      </busconfig>
    '';
  };

  polkitPolicy = pkgs.writeTextFile {
    name = "io.github.HBlanqueto.keel.userd.policy";
    destination = "/share/polkit-1/actions/io.github.HBlanqueto.keel.userd.policy";
    text = builtins.readFile ../services/userd/polkit/io.github.HBlanqueto.keel.userd.policy;
  };

  declaredUsers = lib.mapAttrsToList (name: user: {
    name = name;
    uid = user.uid;
    real_name = user.description or "";
    admin = builtins.elem "wheel" (user.extraGroups or [ ]);
    home = user.home;
    shell = user.shell;
  }) (lib.filterAttrs (_: user: user.isNormalUser) config.users.users);

  managedNames =
    let
      dir = "/persist${cfg.stateDir}/users";
    in
    if builtins ? currentSystem && builtins.pathExists dir then
      map (lib.removeSuffix ".kdl") (
        builtins.filter (name: lib.hasSuffix ".kdl" name) (builtins.attrNames (builtins.readDir dir))
      )
    else
      [ ];

  collisions = builtins.filter (user: builtins.elem user.name managedNames) declaredUsers;

  pamHelper = pkgs.writeShellScript "keel-userd-pam-check" ''
    export PATH=${lib.makeBinPath [ pkgs.mkpasswd ]}
    exec ${cfg.package}/bin/keel-userd "$@"
  '';
in
{
  options.keel.userd = {
    enable = lib.mkEnableOption "keel-userd, gestión de usuarios en runtime";

    package = lib.mkOption {
      type = lib.types.package;
      default = inputs.self.packages.${system}.keel-userd;
      description = "Paquete del daemon keel-userd.";
    };

    stateDir = lib.mkOption {
      type = lib.types.str;
      default = "/var/lib/keel-userd";
      description = "Directorio de estado (KDL de usuarios y journal de jobs).";
    };

    persistence.enable = lib.mkOption {
      type = lib.types.bool;
      default = true;
      description = "Añadir stateDir a environment.persistence.\"/persist\".";
    };

    identity.priority = lib.mkOption {
      type = lib.types.listOf (
        lib.types.enum [
          "homed"
          "userdb"
          "declared"
        ]
      );
      default = [
        "userdb"
        "homed"
        "declared"
      ];
      description = "Orden de preferencia de los backends de identidad.";
    };

    identity.homed.storage = lib.mkOption {
      type = lib.types.enum [
        "directory"
        "subvolume"
      ];
      default = "subvolume";
      description = "Almacenamiento de los homes de homed; nunca luks.";
    };

    identity.userdb.uidRange = lib.mkOption {
      type = lib.types.submodule {
        options = {
          from = lib.mkOption {
            type = lib.types.port;
            default = 30000;
          };
          to = lib.mkOption {
            type = lib.types.port;
            default = 59999;
          };
        };
      };
      default = { };
      description = "Rango de UIDs que asigna el registry de userdb.";
    };

    packages.priority = lib.mkOption {
      type = lib.types.listOf (
        lib.types.enum [
          "nix-profile"
          "home-manager"
          "flatpak"
        ]
      );
      default = [
        "nix-profile"
        "home-manager"
        "flatpak"
      ];
      description = "Orden de preferencia de los backends de paquetes.";
    };
  };

  config = lib.mkIf cfg.enable {
    environment.etc."keel/userd.json".text = builtins.toJSON {
      state_dir = cfg.stateDir;
      userdb_dir = "/etc/userdb";
      identity_priority = cfg.identity.priority;
      package_priority = cfg.packages.priority;
      uid_range = cfg.identity.userdb.uidRange;
      homed_storage = cfg.identity.homed.storage;
      declared_users = declaredUsers;
    };

    assertions = [
      {
        assertion = cfg.identity.userdb.uidRange.from <= cfg.identity.userdb.uidRange.to;
        message = "keel.userd.identity.userdb.uidRange.from debe ser menor o igual que to.";
      }
      {
        assertion = builtins.elem "systemd" config.system.nssDatabases.passwd;
        message = "keel-userd necesita \"systemd\" en system.nssDatabases.passwd.";
      }
      {
        assertion = lib.any (entry: lib.hasInfix "systemd" entry) config.system.nssDatabases.group;
        message = "keel-userd necesita \"systemd\" en system.nssDatabases.group.";
      }
      {
        assertion = collisions == [ ];
        message = "Estos usuarios declarados chocan con usuarios gestionados en ${cfg.stateDir}: ${
          lib.concatStringsSep ", " (map (user: user.name) collisions)
        }";
      }
    ];

    services.homed.enable = lib.mkIf (builtins.elem "homed" cfg.identity.priority) true;
    services.nscd.enable = true;

    security.pam.services = lib.mkIf (builtins.elem "userdb" cfg.identity.priority) {
      login.rules.auth.keel-userd = {
        order = 9000;
        control = "sufficient";
        modulePath = "${pkgs.pam}/lib/security/pam_exec.so";
        args = [
          "expose_authtok"
          "quiet"
          "${pamHelper}"
          "pam-auth"
        ];
      };
      login.rules.account.keel-userd = {
        order = 9000;
        control = "sufficient";
        modulePath = "${pkgs.pam}/lib/security/pam_exec.so";
        args = [
          "quiet"
          "${pamHelper}"
          "pam-account"
        ];
      };
      su.rules.auth.keel-userd = {
        order = 9000;
        control = "sufficient";
        modulePath = "${pkgs.pam}/lib/security/pam_exec.so";
        args = [
          "expose_authtok"
          "quiet"
          "${pamHelper}"
          "pam-auth"
        ];
      };
      su.rules.account.keel-userd = {
        order = 9000;
        control = "sufficient";
        modulePath = "${pkgs.pam}/lib/security/pam_exec.so";
        args = [
          "quiet"
          "${pamHelper}"
          "pam-account"
        ];
      };
      sudo.rules.auth.keel-userd = {
        order = 9000;
        control = "sufficient";
        modulePath = "${pkgs.pam}/lib/security/pam_exec.so";
        args = [
          "expose_authtok"
          "quiet"
          "${pamHelper}"
          "pam-auth"
        ];
      };
      sudo.rules.account.keel-userd = {
        order = 9000;
        control = "sufficient";
        modulePath = "${pkgs.pam}/lib/security/pam_exec.so";
        args = [
          "quiet"
          "${pamHelper}"
          "pam-account"
        ];
      };
    };

    systemd.services.keel-userd = {
      description = "keel-userd user management daemon";
      wantedBy = [ "multi-user.target" ];
      after = [ "dbus.service" ];
      requires = [ "dbus.service" ];

      path = with pkgs; [
        mkpasswd
        systemd
        coreutils
      ];

      serviceConfig = {
        Type = "dbus";
        BusName = "io.github.HBlanqueto.Keel.Userd1";
        ExecStart = "${cfg.package}/bin/keel-userd";
        Restart = "on-failure";
        RestartSec = 2;
      };
    };

    systemd.tmpfiles.rules = [
      "d ${cfg.stateDir} 0700 root root -"
      "d /etc/userdb 0755 root root -"
    ];

    services.dbus.packages = [ dbusPolicy ];
    security.polkit.enable = true;
    environment.systemPackages = [ polkitPolicy ];

    environment.persistence."/persist".directories = lib.mkIf cfg.persistence.enable [
      {
        directory = cfg.stateDir;
        user = "root";
        group = "root";
        mode = "0700";
      }
      {
        directory = "/etc/userdb";
        user = "root";
        group = "root";
        mode = "0755";
      }
      {
        directory = "/var/lib/systemd/home";
        user = "root";
        group = "root";
        mode = "0700";
      }
    ];
  };
}
