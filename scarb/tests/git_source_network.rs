use std::io::{Read, Write};
use std::net::TcpListener;
use std::thread;

use assert_fs::TempDir;
use indoc::indoc;

use scarb_test_support::command::Scarb;
use scarb_test_support::project_builder::{Dep, DepBuilder, ProjectBuilder};

#[test]
fn https_something_happens() {
    thread::scope(|ts| {
        let server = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = server.local_addr().unwrap();
        let port = addr.port();
        ts.spawn(move || {
            let mut conn = server.accept().unwrap().0;
            drop(conn.write(b"1234"));
            drop(conn.shutdown(std::net::Shutdown::Write));
            drop(conn.read(&mut [0; 16]));
        });

        let t = TempDir::new().unwrap();
        ProjectBuilder::start()
            .name("hello")
            .version("1.0.0")
            .dep(
                "dep",
                Dep.with("git", format!("https://127.0.0.1:{port}/foo/bar")),
            )
            .build(&t);

        Scarb::quick_snapbox()
            .arg("fetch")
            .current_dir(&t)
            .assert()
            .failure()
            .stdout_matches(indoc! {r#"
            [..] Updating git repository https://127.0.0.1:[..]/foo/bar
            error: cannot get dependencies of `hello@1.0.0`

            Caused by:
                0: failed to clone into: [..]
                1: failed to clone into: [..]
                2: process did not exit successfully: exit status: 128
            "#});
    });
}

#[test]
fn ssh_something_happens() {
    thread::scope(|ts| {
        let server = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = server.local_addr().unwrap();
        let port = addr.port();
        ts.spawn(move || {
            drop(server.accept().unwrap());
        });

        let t = TempDir::new().unwrap();
        ProjectBuilder::start()
            .name("hello")
            .version("1.0.0")
            .dep(
                "dep",
                Dep.with("git", format!("ssh://127.0.0.1:{port}/foo/bar")),
            )
            .build(&t);

        Scarb::quick_snapbox()
            .arg("fetch")
            .current_dir(&t)
            .assert()
            .failure()
            .stdout_matches(indoc! {r#"
            [..] Updating git repository ssh://127.0.0.1:[..]/foo/bar
            error: cannot get dependencies of `hello@1.0.0`

            Caused by:
                0: failed to clone into: [..]
                1: failed to clone into: [..]
                2: process did not exit successfully: exit status: 128
            "#});
    });
}
