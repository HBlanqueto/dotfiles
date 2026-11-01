#!/bin/sh
set -eu

REPO="github:HBlanqueto/dotfiles"
RELEASES="https://github.com/HBlanqueto/dotfiles/releases/download/latest"
WORK="/tmp/keel-installer"

NIX_CONFIG="experimental-features = nix-command flakes"
export NIX_CONFIG

if [ "$(id -u)" -ne 0 ]; then
    exec sudo -- "$0" "$@"
fi

here=$(CDPATH='' cd -- "$(dirname -- "$0")/.." && pwd)

machine=$(uname -m)
case "$machine" in
    x86_64) arch=x86_64 ;;
    aarch64 | arm64) arch=aarch64 ;;
    *)
        echo "Arquitectura no soportada: $machine (solo x86_64 y aarch64)." >&2
        exit 1
        ;;
esac

rev=""
if [ -r "$here/flake.lock" ]; then
    rev=$(nix eval --impure --raw --expr \
        "(builtins.fromJSON (builtins.readFile $here/flake.lock)).nodes.nixpkgs.locked.rev" \
        2>/dev/null || true)
fi
if [ -n "$rev" ]; then
    nixpkgs="github:nixos/nixpkgs/$rev"
else
    nixpkgs="nixpkgs"
fi

# shellcheck disable=SC2086
tool_pkgs="$nixpkgs#disko $nixpkgs#mkpasswd $nixpkgs#sbctl $nixpkgs#nixos-facter"

mkdir -p "$WORK"
binary="$WORK/nixdots-installer-$arch"

fetch() {
    if command -v curl >/dev/null 2>&1; then
        curl -fsSL "$1" -o "$2"
    elif command -v wget >/dev/null 2>&1; then
        wget -qO "$2" "$1"
    else
        echo "Se necesita curl o wget para descargar el instalador." >&2
        return 1
    fi
}

verified=0
if fetch "$RELEASES/nixdots-installer-$arch" "$binary" &&
    fetch "$RELEASES/SHA256SUMS" "$WORK/SHA256SUMS"; then
    grep " nixdots-installer-$arch\$" "$WORK/SHA256SUMS" >"$WORK/SHA256SUMS.$arch" || true
    if [ -s "$WORK/SHA256SUMS.$arch" ] &&
        (cd "$WORK" && sha256sum -c "SHA256SUMS.$arch"); then
        verified=1
    fi
fi

if [ "$verified" -ne 1 ]; then
    echo "No se pudo descargar o verificar el instalador precompilado." >&2
    echo "Se puede compilar desde el código con Nix: bajará el toolchain de Rust" >&2
    echo "y ~280 crates, puede tardar bastante y ocupar varios GiB en RAM." >&2
    printf '¿Compilarlo ahora con Nix y ejecutarlo? [s/N] ' >&2
    read -r reply || reply=n
    case "$reply" in
        s | S | y | Y) exec nix run "$REPO#installer" -- --install "$@" ;;
        *)
            echo "Alternativa: sudo nix run $REPO#installer -- --install" >&2
            exit 1
            ;;
    esac
fi
chmod +x "$binary"

if [ "${TERM:-}" = linux ] && [ -z "${KEEL_NO_KMSCON:-}" ]; then
    case "$(tty 2>/dev/null || true)" in
        /dev/tty*)
            palette_file="$here/kits/neptunia/generated/kmscon-palette"
            if [ -r "$palette_file" ]; then
                palette=$(cat "$palette_file")

                fontdir=$(nix build --no-link --print-out-paths \
                    "$nixpkgs#nerd-fonts.jetbrains-mono" 2>/dev/null || true)
                if [ -n "$fontdir" ]; then
                    FONTCONFIG_FILE="$WORK/fonts.conf"
                    export FONTCONFIG_FILE
                    cat >"$FONTCONFIG_FILE" <<EOF
<?xml version="1.0"?>
<!DOCTYPE fontconfig SYSTEM "urn:fontconfig:fonts.dtd">
<fontconfig>
  <dir>$fontdir/share/fonts</dir>
  <cachedir>$WORK/fontcache</cachedir>
</fontconfig>
EOF
                fi

                hwaccel=
                if [ -e /dev/dri/renderD128 ]; then hwaccel=--hwaccel; fi

                rm -f "$WORK/started" "$WORK/exit-code"

                # shellcheck disable=SC2086
                nix --extra-experimental-features 'nix-command flakes' \
                    shell "$nixpkgs#kmscon" "$nixpkgs#nerd-fonts.jetbrains-mono" $tool_pkgs -c \
                    kmscon --vt auto --switchvt $hwaccel \
                    --font-name "JetBrainsMono Nerd Font Mono" --font-size 14 \
                    $palette --login -- "$binary" --install "$@" || true

                if [ -e "$WORK/started" ]; then
                    code=$(cat "$WORK/exit-code" 2>/dev/null || true)
                    case "$code" in
                        '' | *[!0-9]*) code=1 ;;
                    esac
                    exit "$code"
                fi
                echo "kmscon no pudo iniciar; se usa el modo ASCII." >&2
            fi
            ;;
    esac
fi

# shellcheck disable=SC2086
exec nix --extra-experimental-features 'nix-command flakes' \
    shell $tool_pkgs -c "$binary" --install "$@"
