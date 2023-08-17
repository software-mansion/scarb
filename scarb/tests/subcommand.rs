use std::io::Read;
use std::net::TcpListener;
use std::process::{Child, Stdio};
use std::{env, io};

use assert_fs::TempDir;
use indoc::{formatdoc, indoc};

use scarb_test_support::command::Scarb;
use scarb_test_support::filesystem::{path_with_temp_dir, write_script, write_simple_hello_script};

#[test]
#[cfg_attr(
    not(target_family = "unix"),
    ignore = "This test should write a Rust code, because currently it only assumes Unix."
)]
fn subcommand() {
    let t = assert_fs::TempDir::new().unwrap();
    write_simple_hello_script("hello", &t);

    Scarb::quick_snapbox()
        .args(["hello", "beautiful", "world"])
        .env("PATH", path_with_temp_dir(&t))
        .assert()
        .success()
        .stdout_eq("Hello beautiful world\n");
}

#[test]
#[cfg_attr(
    not(target_family = "unix"),
    ignore = "This test should write a Rust code, because currently it only assumes Unix."
)]
fn list_commands_e2e() {
    let t = TempDir::new().unwrap();
    write_simple_hello_script("hello", &t);

    let cmd = Scarb::quick_snapbox()
        .args(["commands"])
        .env("PATH", path_with_temp_dir(&t))
        .assert()
        .success();
    let output = cmd.get_output().stdout.clone();
    let stdout = String::from_utf8(output).unwrap();
    assert!(stdout.starts_with("Installed Commands:\n"))
}

#[test]
#[cfg(unix)]
fn env_variables_are_passed() {
    let t = TempDir::new().unwrap();
    write_script(
        "env",
        indoc! {
            r#"
            #!/bin/bash
            required=(
                PATH
                SCARB
                SCARB_CACHE
                SCARB_CONFIG
                SCARB_TARGET_DIR
                SCARB_PROFILE
                SCARB_MANIFEST_PATH
                SCARB_UI_VERBOSITY
            )
            for v in "${required[@]}"; do
                if test -z "${!v}"
                then
                    echo "Variable $v is not set!"
                    exit 1
                fi
            done
            "#
        },
        &t,
    );

    Scarb::quick_snapbox()
        .args(["env"])
        .env("PATH", path_with_temp_dir(&t))
        .assert()
        .success();
}

#[test]
#[cfg(unix)]
fn env_scarb_log_is_passed_verbatim() {
    let t = TempDir::new().unwrap();
    write_script(
        "env",
        indoc! {
            r#"
            #!/usr/bin/env bash
            if [[ "$SCARB_LOG" != "test=filter" ]]
            then
                echo "Variable SCARB_LOG has incorrect value $SCARB_LOG!"
                exit 1
            fi
            if [[ "$SCARB_UI_VERBOSITY" != "verbose" ]]
            then
                echo "Variable SCARB_UI_VERBOSITY has incorrect value $SCARB_UI_VERBOSITY!"
                exit 1
            fi
            "#
        },
        &t,
    );

    Scarb::quick_snapbox()
        .args(["-vvvv", "env"])
        .env("PATH", path_with_temp_dir(&t))
        .env("SCARB_LOG", "test=filter")
        .assert()
        .success();
}

// TODO(#129): Fix this test.
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

    let mut child = Scarb::new()
        .std()
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

#[test]
#[cfg_attr(
    not(target_family = "unix"),
    ignore = "This test should write a Rust code, because currently it only assumes Unix."
)]
fn can_find_scarb_directory_scripts_without_path() {
    let t = assert_fs::TempDir::new().unwrap();
    write_simple_hello_script("hello", &t);

    // Set scarb path to folder containing hello script
    let scarb_path = t
        .path()
        .to_path_buf()
        .join("scarb-hello")
        .to_string_lossy()
        .to_string();
    Scarb::quick_snapbox()
        .env("SCARB", scarb_path)
        .args(["hello", "beautiful", "world"])
        .assert()
        .success()
        .stdout_eq("Hello beautiful world\n");
}

#[test]
fn can_list_scarb_directory_scripts() {
    let t = assert_fs::TempDir::new().unwrap();
    write_simple_hello_script("hello", &t);

    // Set scarb path to folder containing hello script
    let scarb_path = t
        .path()
        .to_path_buf()
        .join(format!("scarb-hello{}", env::consts::EXE_SUFFIX))
        .to_string_lossy()
        .to_string();
    let cmd = Scarb::quick_snapbox()
        .env("SCARB", scarb_path)
        .args(["commands"])
        .assert()
        .success();
    let output = cmd.get_output().stdout.clone();
    let stdout = String::from_utf8(output).unwrap();
    assert!(stdout.contains("hello"))
}
