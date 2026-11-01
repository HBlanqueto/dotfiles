# System-wide locale, time zone, keyboard, console and account settings.
{
  config,
  lib,
  pkgs,
  inputs,
  data,
  ...
}:

let
  inherit (data) root;

  inherit (data.settings)
    timeZone
    defaultLocale
    keyboardLayout
    keyboardVariant
    ;
  region = data.settings.region or null;
  timeZoneAuto = data.settings.timeZoneAuto or false;

  autoLoginUser = data.machine.autoLoginUser or null;

  escapedPretty = lib.replaceStrings [ "\\" "\"" ] [ "\\\\" "\\\"" ] (
    data.machine.prettyHostName or ""
  );

  normalUsers = builtins.filter (u: u.isNormalUser) (builtins.attrValues config.users.users);
  missingHashes = builtins.filter (
    u: (u.hashedPasswordFile or null) != null && !(data.exists u.hashedPasswordFile)
  ) normalUsers;
in

{
  assertions = [
    {
      assertion = !(builtins ? currentSystem) || missingHashes == [ ];
      message =
        "Falta el archivo de contraseña de: "
        + builtins.concatStringsSep ", " (
          map (u: "${u.name} -> ${root}${toString u.hashedPasswordFile}") missingHashes
        )
        + ". Créalo con `mkpasswd -m yescrypt` antes de reconstruir.";
    }
  ];

  time.timeZone = lib.mkIf (!timeZoneAuto) timeZone;

  i18n = {
    inherit defaultLocale;
    supportedLocales = lib.unique [
      "${defaultLocale}/UTF-8"
      "en_US.UTF-8/UTF-8"
      "C.UTF-8/UTF-8"
    ];
    extraLocaleSettings = lib.mkIf (region != null && region != defaultLocale) {
      LC_TIME = region;
      LC_MONETARY = region;
      LC_NUMERIC = region;
      LC_MEASUREMENT = region;
      LC_PAPER = region;
      LC_ADDRESS = region;
      LC_TELEPHONE = region;
    };
  };

  programs = {
    dconf.enable = true;

    fish = {
      enable = true;
      shellAliases = {
        delgen = "sudo nix-collect-garbage --delete-older-than 1d";
        nix-update = "nixdots-rebuild sw";
        flake-update-rb = "nixdots-rebuild update-rb";
        flake-update-sw = "nixdots-rebuild update-sw";
      };
    };
  };

  networking.hostName = data.hostName;

  users = {
    defaultUserShell = pkgs.fish;
    mutableUsers = false;
  };

  services = {
    automatic-timezoned.enable = timeZoneAuto;
    displayManager.autoLogin = lib.mkIf (autoLoginUser != null) {
      enable = true;
      user = autoLoginUser;
    };
    xserver.xkb = {
      layout = keyboardLayout;
      variant = keyboardVariant;
    };
  };

  environment = {
    binsh = "${pkgs.dash}/bin/dash";

    etc."machine-info" = lib.mkIf (data.machine ? prettyHostName) {
      text = ''
        PRETTY_HOSTNAME="${escapedPretty}"
      '';
    };

    systemPackages = with pkgs; [
      comma

      inputs.home.packages.${pkgs.stdenv.hostPlatform.system}.home-manager

      inputs.self.packages.${pkgs.stdenv.hostPlatform.system}.nixdots-rebuild

      wget
      curl

      neovim
      unzip
      tree
      jq
    ];
  };
}
