use std::ffi::OsString;
use std::io::Read;
use std::net::TcpListener;
use std::path::Path;
use std::process::{Child, Stdio};
use std::{env, fs, io, iter, process};

use assert_fs::prelude::*;
use assert_fs::TempDir;
use indoc::formatdoc;
use snapbox::cmd::{cargo_bin, Command};

#[test]
fn subcommand() {
    let t = assert_fs::TempDir::new().unwrap();
    write_script(
        "hello",
        &formatdoc!(
            r#"
            #!/usr/bin/env python3
            import sys
            print("Hello", *sys.argv[1:])
            "#
        ),
        &t,
    );

    Command::new(cargo_bin!("murek"))
        .args(["hello", "beautiful", "world"])
        .env("PATH", path_with_temp_dir(&t))
        .assert()
        .success()
        .stdout_eq("Hello beautiful world\n");
}

// TODO(mkaput): Fix this test.
#[test]
#[ignore] // something doesn't work here
fn ctrl_c_kills_everyone() {
    let t = assert_fs::TempDir::new().unwrap();
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();

    write_script(
        "hang-on-tcp",
        &{
            let addr = listener.local_addr().unwrap();
            let ip = addr.ip();
            let port = addr.port();
            formatdoc!(
                r#"
                #!/usr/bin/env python3
                import socket
                sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
                sock.connect(("{ip}", {port}))
                sock.recv(10)
                raise Exception("recv should never return")
                "#
            )
        },
        &t,
    );

    let mut child = process::Command::new(cargo_bin!("murek"))
        .arg("hang-on-tcp")
        .env("PATH", path_with_temp_dir(&t))
        .stdin(Stdio::piped())
        .spawn()
        .unwrap();

    let (mut sock, _) = listener.accept().unwrap();
    ctrl_c(&mut child);

    assert!(!child.wait().unwrap().success());
    match sock.read(&mut [0; 10]) {
        Ok(n) => assert_eq!(n, 0),
        Err(e) => assert_eq!(e.kind(), io::ErrorKind::ConnectionReset),
    }
}

fn write_script(name: &str, script_source: &str, t: &TempDir) {
    let script = t.child(format!("murek-{name}{}", env::consts::EXE_SUFFIX));
    script.write_str(script_source).unwrap();
    make_executable(script.path());
}

fn path_with_temp_dir(t: &TempDir) -> OsString {
    let script_path = iter::once(t.path().to_path_buf());
    let os_path = env::var_os("PATH").unwrap();
    let other_paths = env::split_paths(&os_path);
    env::join_paths(script_path.chain(other_paths)).unwrap()
}

#[cfg(unix)]
fn ctrl_c(child: &mut Child) {
    let r = unsafe { libc::kill(child.id() as libc::pid_t, libc::SIGINT) };
    if r < 0 {
        panic!("failed to kill: {}", io::Error::last_os_error());
    }
}

#[cfg(windows)]
fn ctrl_c(child: &mut Child) {
    child.kill().unwrap();
}

#[cfg(unix)]
fn make_executable(path: &Path) {
    use std::os::unix::prelude::*;
    let mut perms = fs::metadata(path).unwrap().permissions();
    perms.set_mode(perms.mode() | 0o700);
    fs::set_permissions(path, perms).unwrap();
}

#[cfg(windows)]
fn make_executable(path: &Path) {}
