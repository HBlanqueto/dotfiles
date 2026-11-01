{ inputs, ... }:

{
  imports = [ inputs.malaysia.nixosModules.default ];

  # MalaysiaDreams como sesión adicional en GDM; GNOME sigue siendo la sesión
  # por defecto. Esto activa también el reemplazo de libadwaita parcheada para
  # el menú global (ADR 0052).
  programs.malaysia.enable = true;
}
