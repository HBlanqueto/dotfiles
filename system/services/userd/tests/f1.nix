# nixosTest de F1a: identidad homed/declared y login por PAM.
{
  pkgs,
  inputs,
  daemon,
}:

pkgs.testers.nixosTest {
  name = "keel-userd-f1";

  nodes.machine =
    {
      config,
      lib,
      pkgs,
      ...
    }:
    {
      imports = [
        inputs.impermanence.nixosModules.impermanence
        ../../../core/userd.nix
      ];

      _module.args.inputs = inputs;

      keel.userd = {
        enable = true;
        package = daemon;
        persistence.enable = true;
        # El test corre sobre ext4; homed necesita btrfs para `subvolume`.
        identity.homed.storage = "directory";
      };

      services.homed.promptOnFirstBoot = false;

      # Servicio PAM que simula el login gráfico; no arrancamos GDM para no
      # engordar la VM. Las reglas por defecto incluyen pam_systemd_home.
      security.pam.services.gdm-password = {
        unixAuth = true;
        startSession = true;
      };
      security.polkit.enablePkexecWrapper = true;

      environment.systemPackages = [
        pkgs.util-linux
        pkgs.expect
        pkgs.pamtester
      ];

      users.users.declared = {
        isNormalUser = true;
        uid = 1000;
        description = "Declared Test";
        home = "/home/declared";
        shell = "/run/current-system/sw/bin/bash";
      };

      system.stateVersion = "26.05";
    };

  testScript = ''
    bus = "io.github.HBlanqueto.Keel.Userd1"
    manager = "io.github.HBlanqueto.Keel.Userd1.Manager"
    job_iface = "io.github.HBlanqueto.Keel.Userd1.Job"
    user_iface = "io.github.HBlanqueto.Keel.Userd1.User"
    root = "/io/github/HBlanqueto/Keel/Userd1"

    def call(method, args=""):
        return machine.succeed(
            f"busctl --system call {bus} {root} {manager} {method} {args}".strip()
        )

    def path_of(output):
        return output.split('"')[1]

    def wait_job(path):
        try:
            machine.wait_until_succeeds(
                f"busctl --system get-property {bus} {path} {job_iface} State"
                " | grep -q succeeded",
                timeout=120,
            )
        except Exception:
            print("JOB ERROR:", machine.succeed(
                f"busctl --system get-property {bus} {path} {job_iface} Error"
            ))
            print("JOB STATE:", machine.succeed(
                f"busctl --system get-property {bus} {path} {job_iface} State"
            ))
            raise

    def create(name, options):
        return path_of(call("CreateUser", f'sa{{sv}} "{name}" {options}'))

    def find(name):
        return path_of(call("FindUserByName", f's "{name}"'))

    def prop(path, iface, name):
        return machine.succeed(
            f"busctl --system get-property {bus} {path} {iface} {name}"
        )

    def run_expect(user, password, command):
        """Ejecuta `su - user -c command` con expect y devuelve (rc, output)."""
        expect_block = (
            '-re {[Pp]assword[^:]*:} { send "' + password + '\\r"; exp_continue } '
            '"Authentication is required" { exp_continue } '
            '"Sorry, try again" { close; wait; exit 4 } '
            '"not authorized" { close; wait; exit 5 } '
            '"is not in the sudoers file" { close; wait; exit 5 } '
            'eof { wait; exit 2 } '
            'timeout { close; wait; exit 6 }'
        )
        machine.succeed(
            "set +e; "
            "expect -c 'set timeout 30; "
            f'spawn su - {user} -c "{command}"; '
            f"expect {expect_block}; "
            "catch wait result; exit [lindex $result 3]' > /tmp/run.out 2>&1; "
            "echo $? > /tmp/run.rc"
        )
        rc = int(machine.succeed("cat /tmp/run.rc").strip())
        output = machine.succeed("cat /tmp/run.out")
        return rc, output

    def console_start(tty, user, password):
        machine.send_key(f"alt-f{tty}")
        machine.wait_until_succeeds(f"[ $(fgconsole) = {tty} ]")
        machine.wait_for_unit(f"getty@tty{tty}.service")
        machine.wait_until_succeeds(f"pgrep -f 'agetty.*tty{tty}'")
        machine.wait_until_tty_matches(str(tty), "login: ")
        machine.send_chars(user, 0.05)
        machine.send_key("ret")
        machine.wait_until_tty_matches(str(tty), "Password: ", timeout=30)
        machine.send_chars(password, 0.05)
        machine.send_key("ret")
        machine.wait_until_succeeds(f"pgrep -u {user} bash", timeout=30)
        machine.send_chars("clear", 0.05)
        machine.send_key("ret")
        machine.wait_until_tty_matches(str(tty), "$", timeout=10)

    def console_finish(tty, user):
        machine.send_chars("exit", 0.05)
        machine.send_key("ret")
        # sudo/pkexec pueden dejar el shell en un estado raro; nos aseguramos.
        machine.succeed(f"pkill -KILL -u {user} bash || true")
        machine.wait_until_fails(f"pgrep -u {user} bash", timeout=30)
        machine.wait_for_unit(f"getty@tty{tty}.service")
        machine.wait_until_succeeds(f"pgrep -f 'agetty.*tty{tty}'", timeout=30)

    def console_login(tty, user, password):
        console_start(tty, user, password)
        machine.send_chars(f"id -un > /tmp/{user}-tty-login", 0.05)
        machine.send_key("ret")
        machine.wait_for_file(f"/tmp/{user}-tty-login")
        console_finish(tty, user)

    def console_command_ok(tty, user, password, command, marker):
        console_start(tty, user, password)
        machine.send_chars(command, 0.05)
        machine.send_key("ret")
        machine.wait_until_tty_matches(str(tty), "assword", timeout=30)
        machine.send_chars(password, 0.05)
        machine.send_key("ret")
        machine.wait_until_tty_matches(str(tty), marker, timeout=60)
        console_finish(tty, user)

    def console_command_fails(tty, user, password, command):
        console_start(tty, user, password)
        machine.send_chars(f"{command}; echo FAILED", 0.05)
        machine.send_key("ret")
        try:
            machine.wait_until_tty_matches(str(tty), "assword", timeout=10)
            machine.send_chars(password, 0.05)
            machine.send_key("ret")
        except Exception:
            pass
        try:
            machine.wait_until_tty_matches(str(tty), "FAILED", timeout=15)
        except Exception:
            machine.send_key("ctrl-c")
            machine.wait_until_tty_matches(str(tty), "FAILED", timeout=30)
        console_finish(tty, user)

    def pamtester_ok(service, user, password):
        machine.succeed(f"printf '%s\\n' '{password}' | pamtester {service} {user} authenticate")

    def pamtester_fails(service, user, password):
        machine.fail(f"printf '%s\\n' '{password}' | pamtester {service} {user} authenticate")

    def set_password(path, password, old_password):
        machine.succeed(
            f"busctl --system call {bus} {path} {user_iface} SetPassword ss {password} {old_password}"
        )


    machine.start()
    machine.wait_for_unit("keel-userd.service")
    machine.wait_until_succeeds(f"busctl --system status {bus}")

    # NixOS debe añadir pam_systemd_home a todos los servicios que autentican.
    for service in ["login", "gdm-password", "polkit-1", "sudo", "su", "systemd-user"]:
        machine.succeed(f"grep -q pam_systemd_home /etc/pam.d/{service}")

    # userdb sigue existiendo solo para identidad, nunca para login con contraseña.
    machine.succeed(
        f"out=$(busctl --system call {bus} {root} {manager} CreateUser"
        " sa{sv} nologin 2 backend s userdb password s x 2>&1 || true);"
        " echo \"$out\" | grep -qi 'userdb no admite login por contraseña'"
    )

    # Usuario normal de homed: alta, home y login real.
    alice_job = create(
        "alice",
        '2 "real-name" s "Alice" "password" s "alicepw"',
    )
    wait_job(alice_job)
    alice = find("alice")
    assert "homed" in prop(alice, user_iface, "Backend")
    machine.wait_until_succeeds("getent passwd alice")
    machine.succeed("test -d /home/alice.homedir")
    machine.succeed("test -d /var/lib/systemd/home")

    # Login por tty1 y por su.
    console_login(1, "alice", "alicepw")
    assert "alice" in machine.succeed("cat /tmp/alice-tty-login")
    run_expect("alice", "alicepw", "id -un | grep -q alice")

    # PAM para el servicio gráfico.
    pamtester_ok("gdm-password", "alice", "alicepw")
    pamtester_fails("gdm-password", "alice", "wrongpassword")

    # Admin de homed: wheel, sudo y polkit pidiendo contraseña.
    admin_job = create(
        "admin",
        '3 "real-name" s "Admin" "admin" b true "password" s "adminpw"',
    )
    wait_job(admin_job)
    admin = find("admin")
    assert "homed" in prop(admin, user_iface, "Backend")
    machine.succeed("getent group wheel | grep -q admin")
    machine.succeed("id -nG admin | grep -qw wheel")
    machine.succeed("sudo -l -U admin | grep -q ALL")
    console_command_ok(1, "admin", "adminpw", "sudo true && echo SUDO_OK", "SUDO_OK")
    console_command_ok(1, "admin", "adminpw", "/run/wrappers/bin/pkexec true && echo PKEXEC_OK", "PKEXEC_OK")

    # El usuario normal no puede usar sudo ni pkexec.
    console_command_fails(1, "alice", "alicepw", "sudo true")
    console_command_fails(1, "alice", "alicepw", "/run/wrappers/bin/pkexec true")

    # Persistencia del home antes del reinicio.
    run_expect("alice", "alicepw", "echo persist > /home/alice/.persist-test")

    # SetPassword cambia la contraseña y la vieja deja de valer.
    set_password(alice, "newalicepw", "alicepw")
    pamtester_ok("gdm-password", "alice", "newalicepw")
    pamtester_fails("gdm-password", "alice", "alicepw")
    rc, _ = run_expect("alice", "alicepw", "true")
    assert rc != 0, "la contraseña vieja sigue funcionando"
    run_expect("alice", "newalicepw", "id -un | grep -q alice")

    # Reinicio: usuarios, homes y contraseñas siguen funcionando.
    machine.shutdown()
    machine.start()
    machine.wait_for_unit("keel-userd.service")
    machine.wait_for_unit("systemd-homed.service")
    machine.wait_until_succeeds(f"busctl --system status {bus}")
    machine.wait_until_succeeds("getent passwd alice")
    machine.wait_until_succeeds("getent passwd admin")
    machine.succeed("test -d /var/lib/systemd/home")

    console_login(1, "alice", "newalicepw")
    run_expect("alice", "newalicepw", "test -f /home/alice/.persist-test")
    pamtester_ok("gdm-password", "alice", "newalicepw")
    pamtester_fails("gdm-password", "alice", "alicepw")

    console_command_ok(1, "admin", "adminpw", "sudo true && echo SUDO_OK", "SUDO_OK")
    console_command_ok(1, "admin", "adminpw", "/run/wrappers/bin/pkexec true && echo PKEXEC_OK", "PKEXEC_OK")
    console_command_fails(1, "alice", "newalicepw", "sudo true")
    console_command_fails(1, "alice", "newalicepw", "/run/wrappers/bin/pkexec true")

    # Los declarados siguen siendo de solo lectura.
    declared = find("declared")
    assert "false" in prop(declared, user_iface, "Managed")
    machine.succeed(
        f"sh -c 'busctl --system call {bus} {declared} {user_iface} SetPassword ss x x"
        " 2>&1 | grep -qi read-only'"
    )
  '';
}
