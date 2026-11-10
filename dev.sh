#!/bin/sh
set -eu

REPO=$(CDPATH='' cd -- "$(dirname -- "$0")" && pwd)
cd "$REPO"

usage() {
    sed -n '3,12p' "$0" | sed 's/^# \{0,1\}//'
}

# Usage: dev.sh sim installer [--here]
#        dev.sh test [--no-build]
#        dev.sh -h | --help

die() {
    code=$1
    shift
    printf '%s\n' "$*" >&2
    exit "$code"
}

require() {
    command -v "$1" >/dev/null 2>&1 || die 1 "missing $1"
}

here() {
    exec nix develop --command "$@"
}

run_window() {
    class=$1
    watches=$2
    exts=$3
    shift 3

    if [ -z "${WAYLAND_DISPLAY:-}" ] || [ -z "${XDG_RUNTIME_DIR:-}" ]; then
        printf 'No WAYLAND_DISPLAY/XDG_RUNTIME_DIR; running in this terminal.\n' >&2
        here "$@"
    fi

    if pgrep -f -- "--class $class --cwd $REPO " >/dev/null 2>&1; then
        printf 'A %s window for %s is already open (reloads on save).\n' "$class" "$REPO"
        exit 0
    fi

    setsid -f wezterm start --always-new-process --class "$class" \
        --cwd "$REPO" -- nix develop --command \
        watchexec -r -c $watches -e "$exts" -- "$@"

    printf 'Simulation window launched (%s).\n' "$class"
}

sim_installer() {
    if [ "${1:-}" = "--here" ] || [ "${1:-}" = "-t" ]; then
        here cargo run --manifest-path installer/Cargo.toml -- --repo "$REPO"
    fi
    run_window installer-dev \
        "-w installer/src -w kits/nixcore/src -w installer/Cargo.toml -w kits/nixcore/Cargo.toml -w Cargo.toml" \
        rs,toml \
        cargo run --manifest-path installer/Cargo.toml -- --repo "$REPO"
}

run_test() {
    build=1
    for arg in "$@"; do
        case "$arg" in
            --no-build) build=0 ;;
            -h | --help)
                printf 'usage: %s test [--no-build]\n' "$0"
                exit 0
                ;;
            *) die 2 "unknown argument: $arg" ;;
        esac
    done

    require python3

    say() { printf '\n\033[1m==> %s\033[0m\n' "$*"; }
    fail() { printf '\033[31mFAIL: %s\033[0m\n' "$*" >&2; exit 1; }
    ok() { printf '\033[32mok\033[0m  %s\n' "$*"; }

    say "cargo clippy -D warnings"
    nix develop -c cargo clippy --workspace --all-targets -- -D warnings
    ok "clippy"

    say "cargo test"
    nix develop -c cargo test --workspace
    ok "tests"

    say "nix flake check --impure --no-build"
    nix flake check --impure --no-build
    ok "flake"

    if [ "$build" -eq 1 ]; then
        say "nix build .#installer"
        nix build .#installer
    fi

    installer="$REPO/result/bin/install-nixos"
    [ -x "$installer" ] || fail "$installer not found (missing --no-build?)"

    driver() {
        binpath=$1
        repo=$2
        python3 - "$binpath" "$repo" <<'PY'
import os, pty, sys, time, fcntl, termios, struct, select

binpath, repo = sys.argv[1], sys.argv[2]
args = [binpath, "--repo", repo]

pid, fd = pty.fork()
if pid == 0:
    os.environ["TERM"] = os.environ.get("PTY_TERM", "xterm-256color")
    os.execv(binpath, args)
fcntl.ioctl(fd, termios.TIOCSWINSZ, struct.pack("HHHH", 40, 110, 0, 0))

buf = bytearray()
def pump(sec):
    end = time.time() + sec
    while time.time() < end:
        r, _, _ = select.select([fd], [], [], 0.1)
        if r:
            try:
                d = os.read(fd, 65536)
            except OSError:
                return
            if not d:
                return
            buf.extend(d)

def send(s, wait=0.5):
    os.write(fd, s)
    pump(wait)

def wait_for(marker, timeout=15):
    return wait_for_any([marker], timeout)

def wait_for_any(markers, timeout=15):
    end = time.time() + timeout
    while time.time() < end:
        pump(0.2)
        if any(m.encode() in buf for m in markers):
            return True
    return False

wait_for("Continuar")
send(b"\r")
wait_for("Para continuar con la instalación")
send(b"\r")
wait_for("Leí y acepto")
send(b"\r")                       # Acepto -> Red
wait_for("Red")
send(b"\r")
wait_for("contraseña")
for c in b"contrasena-larga":
    send(bytes([c]), 0.05)
send(b"\r")                       # Conectando
wait_for("conectando a", 12)      # al conectar avanza solo a Lenguaje
wait_for("Lenguaje")
send(b"\x1b[B")
send(b"\r")
wait_for("Teclado")
send(b"\x1b[B")
send(b"\r")                       # elegir layout -> botones
send(b"\r")                       # Continuar -> Zona horaria
wait_for("Zona horaria")
wait_for_any(["Africa/", "America/", "Europe/", "UTC"])  # lista cargada
send(b"\x1b[B")
send(b"\r")                       # -> Equipo
wait_for("Equipo")
send(b"\r")                       # -> Disco
wait_for("Disco")
send(b"\r")                       # -> Cifrado
wait_for("Cifrado")
for c in b"frase-larga-123":
    send(bytes([c]), 0.05)
send(b"\x1b[B")                   # -> Confirmar
for c in b"frase-larga-123":
    send(bytes([c]), 0.05)
send(b"\x1b[B")                   # -> TPM (si lo hay) o «La anoté»
if b"Desbloquear con TPM" in buf:
    send(b"\x1b[B")               # -> La anoté
send(b" ")                        # marcar
send(b"\x1b[B")                   # -> botones
send(b"\r")                       # Continuar -> Resumen
wait_for("Resumen")
send(b"\r")                       # Resumen -> Instalando (dry run ~20 s)
wait_for("Instalando")
wait_for("Reiniciar", 40)         # Instalando -> Listo (auto)
send(b"\r", 1.0)                  # Listo -> Reiniciar (dry run sale)

pump(0.5)
try:
    os.close(fd)
except OSError:
    pass
sys.stdout.buffer.write(bytes(buf))
PY
    }

    MISSING=""
    capture() {
        bin=$1
        out=$(mktemp)
        attempt=1
        while [ "$attempt" -le 3 ]; do
            driver "$bin" "$REPO" >"$out"
            missing=""
            while IFS= read -r want; do
                [ -z "$want" ] && continue
                grep -aq -- "$want" "$out" || { missing="$want"; break; }
            done <<'EOF'
Welcome
Para continuar con la instalación
Red
Lenguaje
Teclado
Zona horaria
Equipo
Disco
Cifrado
Resumen
Instalando
Reiniciar
simulación
conectando a
Acepto
clave de recuperación
La anoté en un lugar seguro
EOF
            if [ -z "$missing" ]; then
                rm -f "$out"
                return 0
            fi
            attempt=$((attempt + 1))
        done
        MISSING="$missing"
        rm -f "$out"
        return 1
    }

    say "simulation: full walkthrough"
    if capture "$installer"; then
        ok "installer: 17 checks"
    else
        fail "installer: missing «$MISSING»"
    fi

    say "palette in TERM=linux (OSC P / OSC R)"
    linux=$(mktemp)
    PTY_TERM=linux driver "$installer" "$REPO" >"$linux"
    grep -aqU "$(printf '\033]P4')" "$linux" || fail "OSC P slot 4 not programmed in TERM=linux"
    grep -aqU "$(printf '\033]R')" "$linux" || fail "palette reset (OSC R) not restored"
    rm -f "$linux"
    ok "TERM=linux: 16 slots + reset"

    printf '\n\033[32mALL OK\033[0m\n'
}

cmd=${1:-}
case "$cmd" in
    sim)
        shift
        what=${1:-}
        [ -n "$what" ] || die 2 "usage: $0 sim installer [--here]"
        shift
        case "$what" in
            installer) sim_installer "$@" ;;
            *) die 2 "usage: $0 sim installer [--here]" ;;
        esac
        ;;
    test) shift; run_test "$@" ;;
    -h | --help | "") usage ;;
    *) usage >&2; exit 2 ;;
esac
