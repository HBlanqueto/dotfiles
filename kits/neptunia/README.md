# Neptunia

## Objetivo y alcance

Neptunia es la única fuente de colores de Keel. Cubre la paleta de terminal y
lo que se deriva de ella: kmscon, el instalador, wezterm, yazi, eza y VS Code.
Este documento es para quien cambia un color.

```
kits/neptunia/
├── default.nix        expone colors, filecolors y vscode
├── neptunia.nix       paleta de terminal (colors)
├── filecolors.nix     colores por tipo de archivo (eza/yazi)
├── vscode.nix         tema de VS Code
├── palette.nix        los 16 colores ANSI y los argumentos de kmscon
└── generated/
    └── kmscon-palette argumentos `--palette*` de kmscon, versionados para install.sh
```

## Cambiar un color

1. Edita el valor en `neptunia.nix`.
2. Regenera la paleta de kmscon:

   ```sh
   cp "$(nix build .#kmscon-palette --print-out-paths)" kits/neptunia/generated/kmscon-palette
   ```

3. Comprueba que `generated/` sigue sincronizado:

   ```sh
   nix flake check --impure
   ```

4. Si el color lo usa el instalador, actualiza `installer/src/theme.rs`; su test
   lo compara con `neptunia.nix`.
