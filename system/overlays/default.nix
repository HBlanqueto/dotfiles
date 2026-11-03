final: _: {
  sf-mono-liga-bin = final.callPackage ../packages/sf-mono-liga-bin.nix { };

  consoleet-darwin = final.callPackage ../packages/consoleet-darwin.nix { };

  xnu-console-font = final.callPackage ../packages/xnu-console-font.nix { };

}
