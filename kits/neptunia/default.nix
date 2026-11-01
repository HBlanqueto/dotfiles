# Neptunia design system: translates tokens into each output format.
let
  colors = import ./neptunia.nix;
  tokens = builtins.fromJSON (builtins.readFile ./tokens.json);
  components = builtins.fromJSON (builtins.readFile ./components.json);

  themes = [
    "light"
    "dark"
    "dark-tint"
  ];
  inherit (builtins)
    attrNames
    concatStringsSep
    isAttrs
    isInt
    isFloat
    match
    toString
    ;
  colorNames = attrNames tokens.themes.light;
  lines = f: xs: concatStringsSep "\n" (map f xs);
  num = v: if isFloat v then toString v else toString v;
  ident = s: builtins.replaceStrings [ "-" ] [ "_" ] s;
  firstFamily = f: let parts = builtins.split "," f; in
    builtins.replaceStrings [ "\"" ] [ "" ] (builtins.head parts);
  qmlColor = c:
    if builtins.stringLength c == 9
    then "#${builtins.substring 7 2 c}${builtins.substring 1 6 c}"
    else c;
  layerNames = [
    "window.light"
    "window.dark"
    "window.dark-tint"
    "float.light"
    "float.dark"
    "float.dark-tint"
  ];
  layerValue = name: path:
    let parts = builtins.split "\\." "${name}.${path}";
        clean = builtins.filter builtins.isString parts;
    in builtins.foldl' (acc: k: acc.${k}) tokens.shadow.layers clean;

  flatten =
    prefix: set:
    builtins.concatLists (
      map (
        k:
        let
          v = set.${k};
          name = if prefix == "" then k else "${prefix}-${k}";
          ref = if builtins.isString v then match "(radius|space|size|duration)\\.(.+)" v else null;
        in
        if isAttrs v then
          flatten name v
        else if isInt v || isFloat v then
          [
            {
              inherit name;
              value = v;
              duration = false;
            }
          ]
        else if ref != null then
          [
            {
              inherit name;
              value = tokens.${builtins.head ref}.${builtins.elemAt ref 1};
              duration = builtins.head ref == "duration";
            }
          ]
        else
          [ ]
      ) (attrNames set)
    );
  metrics = flatten "" components.components;
  metricType =
    m:
    if m.duration then
      "duration"
    else if match ".*(weight|columns|lines).*" m.name != null then
      "int"
    else if match ".*(opacity|scale).*" m.name != null then
      "float"
    else
      "length";
  metricValue =
    m:
    let
      t = metricType m;
    in
    if t == "length" then
      "${num m.value}px"
    else if t == "duration" then
      "${num m.value}ms"
    else
      num m.value;

  styleNames = attrNames tokens.font.styles;
  sh = tokens.shadow.slint;

  slint = ''
    // Generado desde kits/neptunia (tokens.json, components.json). No editar a mano:
    // cambia los JSON y regenera con `nix eval --raw -f kits/neptunia slint`.
    export enum NeptuniaMode { light, dark, dark-tint }

    export struct NeptuniaAccent {
        id: string,
        color: color,
        label: string,
    }

    export global Theme {
        in-out property <NeptuniaMode> mode: NeptuniaMode.light;

        // Colores (por tema)
    ${lines (
      n:
      "    out property <color> ${n}: mode == NeptuniaMode.dark ? ${tokens.themes.dark.${n}} : mode == NeptuniaMode.dark-tint ? ${tokens.themes."dark-tint".${n}} : ${tokens.themes.light.${n}};"
    ) colorNames}

        // Redondeo, espacio, tamaños y duraciones
    ${lines (n: "    out property <length> radius-${n}: ${num tokens.radius.${n}}px;") (
      attrNames tokens.radius
    )}
    ${lines (n: "    out property <length> space-${n}: ${num tokens.space.${n}}px;") (
      attrNames tokens.space
    )}
    ${lines (n: "    out property <length> ${n}: ${num tokens.size.${n}}px;") (attrNames tokens.size)}
    ${lines (n: "    out property <duration> dur-${n}: ${num tokens.duration.${n}}ms;") (
      attrNames tokens.duration
    )}

        // Tipografía
        out property <string> font-sans: "${firstFamily tokens.font.families.sans}";
        out property <string> font-mono: "${firstFamily tokens.font.families.mono}";
        out property <string> font-script: "${firstFamily tokens.font.families.script}";
        out property <[int]> font-weights: [${concatStringsSep ", " (map toString tokens.font.weights)}];
    ${lines (
      s: "    out property <length> text-${s}-size: ${num tokens.font.styles.${s}.size}px;"
    ) styleNames}
    ${lines (
      s: "    out property <int> text-${s}-weight: ${num tokens.font.styles.${s}.weight};"
    ) styleNames}

        // Sombras (Slint admite una por elemento)
        out property <length> shadow-window-offset-y: ${num sh.window."offset-y"}px;
        out property <length> shadow-window-blur: ${num sh.window.blur}px;
        out property <color> shadow-window-color: mode == NeptuniaMode.light ? ${sh.window.color.light} : ${sh.window.color.dark};
        out property <length> shadow-float-offset-y: ${num sh.float."offset-y"}px;
        out property <length> shadow-float-blur: ${num sh.float.blur}px;
        out property <color> shadow-float-color: mode == NeptuniaMode.light ? ${sh.float.color.light} : ${sh.float.color.dark};
        out property <length> shadow-knob-offset-y: ${num sh.knob."offset-y"}px;
        out property <length> shadow-knob-blur: ${num sh.knob.blur}px;
        out property <color> shadow-knob-color: ${sh.knob.color};

        // Sombras por capas (los textos CSS siguen arriba).
        out property <float> shadow-window-unfocused-scale: ${num tokens.shadow.layers.window.unfocused-scale};
    ${lines (th: ''
        out property <length> shadow-window-${th}-ambient-blur: ${num tokens.shadow.layers.window.${th}.ambient.blur}px;
        out property <length> shadow-window-${th}-ambient-offset-y: ${num tokens.shadow.layers.window.${th}.ambient.y}px;
        out property <float> shadow-window-${th}-ambient-opacity: ${num tokens.shadow.layers.window.${th}.ambient.opacity};
        out property <length> shadow-window-${th}-contact-blur: ${num tokens.shadow.layers.window.${th}.contact.blur}px;
        out property <length> shadow-window-${th}-contact-offset-y: ${num tokens.shadow.layers.window.${th}.contact.y}px;
        out property <float> shadow-window-${th}-contact-opacity: ${num tokens.shadow.layers.window.${th}.contact.opacity};'') [
      "light"
      "dark"
      "dark-tint"
    ]}
    ${lines (th: ''
        out property <length> shadow-float-${th}-blur: ${num tokens.shadow.layers.float.${th}.blur}px;
        out property <length> shadow-float-${th}-offset-y: ${num tokens.shadow.layers.float.${th}.y}px;
        out property <float> shadow-float-${th}-opacity: ${num tokens.shadow.layers.float.${th}.opacity};'') [
      "light"
      "dark"
      "dark-tint"
    ]}

        // Acentos de sistema (GNOME), seleccionables junto al de Neptunia.
        out property <[NeptuniaAccent]> accents: [
    ${lines (
      a: "        { id: \"${a.id}\", color: ${a.color}, label: \"${a.label}\" },"
    ) tokens.accents}
        ];
    }

    // Medidas de components.json (switch-width, segmented-segment-radius, toolbar-tabs-tab-min-width…)
    export global Metrics {
    ${lines (m: "    out property <${metricType m}> ${m.name}: ${metricValue m};") metrics}
    }
  '';

  cssVars = th: lines (n: "  --${n}: ${tokens.themes.${th}.${n}};") colorNames;
  css = ''
    /* Generado desde kits/neptunia. data-theme="dark" o "dark-tint" cambia el subárbol. */
    :root, [data-theme="light"] {
    ${cssVars "light"}
      --shadow-window: ${tokens.shadow.window.light};
      --shadow-float: ${tokens.shadow.float.light};
    }
    ${lines
      (th: ''
        [data-theme="${th}"] {
        ${cssVars th}
          --shadow-window: ${tokens.shadow.window.${th}};
          --shadow-float: ${tokens.shadow.float.${th}};
        }'')
      [
        "dark"
        "dark-tint"
      ]
    }
    :root {
    ${lines (n: "  --radius-${n}: ${num tokens.radius.${n}}px;") (attrNames tokens.radius)}
    ${lines (n: "  --space-${n}: ${num tokens.space.${n}}px;") (attrNames tokens.space)}
    ${lines (n: "  --${n}: ${num tokens.size.${n}}px;") (attrNames tokens.size)}
    ${lines (n: "  --dur-${n}: ${num tokens.duration.${n}}ms;") (attrNames tokens.duration)}
      --shadow-knob: ${tokens.shadow.knob};
      --font-sans: ${tokens.font.families.sans};
      --font-mono: ${tokens.font.families.mono};
      --font-script: ${tokens.font.families.script};
      --font-weights: ${concatStringsSep " " (map toString tokens.font.weights)};
    ${lines (s: "  --text-${s}-size: ${num tokens.font.styles.${s}.size}px;") styleNames}
    ${lines (s: "  --text-${s}-line-height: ${num tokens.font.styles.${s}.line-height}px;") styleNames}
    ${lines (s: "  --text-${s}-weight: ${num tokens.font.styles.${s}.weight};") styleNames}
    ${lines (th: ''
      --shadow-window-${th}-ambient-blur: ${num tokens.shadow.layers.window.${th}.ambient.blur}px;
      --shadow-window-${th}-ambient-offset-y: ${num tokens.shadow.layers.window.${th}.ambient.y}px;
      --shadow-window-${th}-ambient-opacity: ${num tokens.shadow.layers.window.${th}.ambient.opacity};
      --shadow-window-${th}-contact-blur: ${num tokens.shadow.layers.window.${th}.contact.blur}px;
      --shadow-window-${th}-contact-offset-y: ${num tokens.shadow.layers.window.${th}.contact.y}px;
      --shadow-window-${th}-contact-opacity: ${num tokens.shadow.layers.window.${th}.contact.opacity};
      --shadow-float-${th}-blur: ${num tokens.shadow.layers.float.${th}.blur}px;
      --shadow-float-${th}-offset-y: ${num tokens.shadow.layers.float.${th}.y}px;
      --shadow-float-${th}-opacity: ${num tokens.shadow.layers.float.${th}.opacity};'') [
      "light"
      "dark"
      "dark-tint"
    ]}
      --shadow-window-unfocused-scale: ${num tokens.shadow.layers.window.unfocused-scale};
    ${lines (m: "  --${m.name}: ${metricValue m};") metrics}
    }
  '';

  qml = ''
    // Generado desde kits/neptunia. No editar a mano.
    pragma Singleton
    import QtQuick

    QtObject {
      readonly property string fontSans: "${firstFamily tokens.font.families.sans}"
      readonly property string fontMono: "${firstFamily tokens.font.families.mono}"
      readonly property string fontScript: "${firstFamily tokens.font.families.script}"
      readonly property var fontWeights: [${concatStringsSep ", " (map toString tokens.font.weights)}]
    ${lines (th: lines (n: "  readonly property color ${ident th}_${ident n}: ${qmlColor tokens.themes.${th}.${n}};") colorNames) themes}
    ${lines (n: "  readonly property real radius_${ident n}: ${num tokens.radius.${n}};") (attrNames tokens.radius)}
    ${lines (n: "  readonly property real space_${ident n}: ${num tokens.space.${n}};") (attrNames tokens.space)}
    ${lines (n: "  readonly property real size_${ident n}: ${num tokens.size.${n}};") (attrNames tokens.size)}
    ${lines (n: "  readonly property real duration_${ident n}: ${num tokens.duration.${n}};") (attrNames tokens.duration)}
    ${lines (s: "  readonly property real text_${ident s}_size: ${num tokens.font.styles.${s}.size};") styleNames}
    ${lines (s: "  readonly property real text_${ident s}_line_height: ${num tokens.font.styles.${s}.line-height};") styleNames}
    ${lines (s: "  readonly property int text_${ident s}_weight: ${num tokens.font.styles.${s}.weight};") styleNames}
    ${lines (th: ''
      readonly property real shadow_window_${ident th}_ambient_blur: ${num tokens.shadow.layers.window.${th}.ambient.blur};
      readonly property real shadow_window_${ident th}_ambient_offset_y: ${num tokens.shadow.layers.window.${th}.ambient.y};
      readonly property real shadow_window_${ident th}_ambient_opacity: ${num tokens.shadow.layers.window.${th}.ambient.opacity};
      readonly property real shadow_window_${ident th}_contact_blur: ${num tokens.shadow.layers.window.${th}.contact.blur};
      readonly property real shadow_window_${ident th}_contact_offset_y: ${num tokens.shadow.layers.window.${th}.contact.y};
      readonly property real shadow_window_${ident th}_contact_opacity: ${num tokens.shadow.layers.window.${th}.contact.opacity};
      readonly property real shadow_float_${ident th}_blur: ${num tokens.shadow.layers.float.${th}.blur};
      readonly property real shadow_float_${ident th}_offset_y: ${num tokens.shadow.layers.float.${th}.y};
      readonly property real shadow_float_${ident th}_opacity: ${num tokens.shadow.layers.float.${th}.opacity};'') [
      "light"
      "dark"
      "dark-tint"
    ]}
      readonly property real shadow_window_unfocused_scale: ${num tokens.shadow.layers.window.unfocused-scale};
    ${lines (m: "  readonly property real ${ident m.name}: ${num m.value};") metrics}
    }
  '';

in
rec {
  inherit
    colors
    tokens
    components
    themes
    slint
    css
    qml
    ;

  filecolors = import ./filecolors.nix;
  vscode = import ./vscode.nix { inherit colors; };

  files =
    pkgs:
    pkgs.runCommand "neptunia-design" { } ''
      mkdir -p $out
      cp ${pkgs.writeText "neptunia.slint" slint} $out/neptunia.slint
      cp ${pkgs.writeText "neptunia.css" css} $out/neptunia.css
      cp ${pkgs.writeText "Neptunia.qml" qml} $out/Neptunia.qml
      cp ${./tokens.json} $out/tokens.json
      cp ${./components.json} $out/components.json
    '';
}
