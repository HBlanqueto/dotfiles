# nixosTest de F1a: backends de identidad userdb/homed/declared.
{
  pkgs,
  inputs,
  daemon,
}:

pkgs.testers.nixosTest {
  name = "keel-userd-f1";

  nodes.machine =
    { lib, pkgs, ... }:
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
        identity.homed.storage = "directory";
      };

      services.homed.promptOnFirstBoot = false;
      security.sudo.wheelNeedsPassword = false;
      environment.systemPackages = [
        pkgs.util-linux
        pkgs.expect
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

    def run_as(name, password, command, marker):
        expect_block = (
            f'\"Password:\" {{ send \"{password}\\r\"; exp_continue }} '
            f'\"Login incorrect\" {{ close; wait; exit 4 }} '
            f'\"{marker}\" {{ close; wait; exit 0 }} '
            f'eof {{ wait; exit 2 }} '
            f'timeout {{ close; wait; exit 5 }}'
        )
        machine.succeed(
            "set +e; "
            f"expect -c 'set timeout 30; "
            f"spawn su - {name} -c \"{command}\"; "
            f"expect {expect_block}; "
            "catch wait result; exit [lindex $result 3]' > /tmp/run_as.out 2>&1; "
            "echo $? > /tmp/run_as.rc"
        )
        rc = int(machine.succeed("cat /tmp/run_as.rc").strip())
        output = machine.succeed("cat /tmp/run_as.out")
        return rc, output

    def login_as(name, password):
        rc, output = run_as(name, password, "id -un", name)
        assert rc == 0, output
        assert name in output, output

    def can_sudo(name, password):
        rc, output = run_as(name, password, "sudo -n true && echo SUDO_OK", "SUDO_OK")
        return rc == 0 and "SUDO_OK" in output


    machine.start()
    machine.wait_for_unit("keel-userd.service")
    machine.wait_until_succeeds(f"busctl --system status {bus}")

    # Usuario normal de userdb: alta, home y login real.
    alice_job = create(
        "alice",
        '2 "real-name" s "Alice" "password" s "alicepw"',
    )
    wait_job(alice_job)
    alice = find("alice")
    assert "userdb" in prop(alice, user_iface, "Backend")
    machine.succeed("getent passwd alice")
    machine.succeed("stat -c %U:%G /home/alice | grep -q alice:alice")
    machine.succeed("test -f /etc/userdb/alice.user")
    machine.succeed("test -f /etc/userdb/alice.user-privileged")
    machine.succeed("test -L /etc/userdb/30000.user")
    machine.succeed("test -f /etc/userdb/alice:video.membership")
    login_as("alice", "alicepw")

    # Admin de userdb: wheel vía userdb y sudo/polkit.
    admin_job = create(
        "admin",
        '3 "real-name" s "Admin" "admin" b true "password" s "adminpw"',
    )
    wait_job(admin_job)
    admin = find("admin")
    machine.succeed("test -f /etc/userdb/admin:wheel.membership")
    machine.succeed("getent group wheel | grep -q admin")
    machine.succeed("id -nG admin | grep -qw wheel")
    machine.succeed("sudo -l -U admin | grep -q ALL")
    login_as("admin", "adminpw")
    assert can_sudo("admin", "adminpw")
    machine.succeed("grep -q 'unix-group:wheel' /etc/polkit-1/rules.d/10-nixos.rules")

    # El usuario normal no puede usar sudo.
    assert not can_sudo("alice", "alicepw")

    # Usuario de homed con storage directory y login real.
    bob_job = create(
        "bob",
        '3 "real-name" s "Bob" "backend" s "homed" "password" s "bobpassword"',
    )
    wait_job(bob_job)
    bob = find("bob")
    assert "homed" in prop(bob, user_iface, "Backend")
    machine.succeed("getent passwd bob")
    login_as("bob", "bobpassword")


    # Los usuarios declarados son de solo lectura.
    declared = find("declared")
    assert "false" in prop(declared, user_iface, "Managed")
    machine.succeed(
        f"sh -c 'busctl --system call {bus} {declared} {user_iface} SetPassword s x"
        " 2>&1 | grep -qi read-only'"
    )
    machine.succeed(
        f"sh -c 'busctl --system call {bus} {root} {manager} ArchiveUser ob {declared} false"
        " 2>&1 | grep -qi read-only'"
    )

    # userdb no disponible: sin backend explícito cae en homed.
    machine.succeed("mount -t tmpfs -o ro tmpfs /etc/userdb")
    backends = call("ListBackends")
    assert '"userdb" "userdb" false' in backends, backends
    carol_job = create(
        "carol",
        '2 "real-name" s "Carol" "password" s "carolpassword"',
    )
    wait_job(carol_job)
    carol = find("carol")
    assert "homed" in prop(carol, user_iface, "Backend")
    login_as("carol", "carolpassword")
    machine.succeed("umount /etc/userdb")

    # Archivar un usuario userdb y comprobar que su UID no se reutiliza.
    archive_job = create("archive", "0")
    wait_job(archive_job)
    archive = find("archive")
    archived_uid = prop(archive, user_iface, "Uid").split()[-1]
    archive_op = path_of(
        call("ArchiveUser", f"ob {archive} false")
    )
    wait_job(archive_op)
    machine.fail("test -e /etc/userdb/archive.user")
    machine.succeed("ls -d /var/lib/keel-userd/archive/archive-* >/dev/null")
    machine.fail(f"busctl --system call {bus} {root} {manager} FindUserByName s archive")

    next_job = create("next", "0")
    wait_job(next_job)
    next_user = find("next")
    next_uid = prop(next_user, user_iface, "Uid").split()[-1]
    assert next_uid != archived_uid, (next_uid, archived_uid)

    # Reinicio: los usuarios de userdb y homed siguen existiendo y entrando.
    machine.shutdown()
    machine.start()
    machine.wait_for_unit("keel-userd.service")
    machine.wait_for_unit("nscd.service")
    machine.wait_until_succeeds(f"busctl --system status {bus}")
    machine.wait_until_succeeds("getent passwd alice")
    machine.wait_until_succeeds("getent passwd bob")
    login_as("alice", "alicepw")
    login_as("bob", "bobpassword")
  '';
}
