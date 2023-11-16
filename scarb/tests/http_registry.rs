use std::fs;
use std::time::Duration;

use assert_fs::prelude::*;
use assert_fs::TempDir;
use expect_test::expect;
use indoc::indoc;

use scarb_test_support::command::Scarb;
use scarb_test_support::project_builder::{Dep, DepBuilder, ProjectBuilder};
use scarb_test_support::registry::http::HttpRegistry;

#[test]
fn usage() {
    let mut registry = HttpRegistry::serve();
    registry.publish(|t| {
        ProjectBuilder::start()
            .name("bar")
            .version("1.0.0")
            .lib_cairo(r#"fn f() -> felt252 { 0 }"#)
            .build(t);
    });

    let t = TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("foo")
        .version("0.1.0")
        .dep("bar", Dep.version("1").registry(&registry))
        .lib_cairo(r#"fn f() -> felt252 { bar::f() }"#)
        .build(&t);

    // FIXME(mkaput): Why are verbose statuses not appearing here?
    Scarb::quick_snapbox()
        .arg("fetch")
        .current_dir(&t)
        .timeout(Duration::from_secs(10))
        .assert()
        .success()
        .stdout_matches(indoc! {r#"
        [..] Downloading bar v1.0.0 ([..])
        "#});

    let expected = expect![["
        GET /config.json
        accept: */*
        accept-encoding: gzip, br, deflate
        host: ...
        user-agent: ...

        200 OK
        accept-ranges: bytes
        content-length: ...
        content-type: application/json
        etag: ...
        last-modified: ...

        ###

        GET /index/3/b/bar.json
        accept: */*
        accept-encoding: gzip, br, deflate
        host: ...
        user-agent: ...

        200 OK
        accept-ranges: bytes
        content-length: ...
        content-type: application/json
        etag: ...
        last-modified: ...

        ###

        GET /bar-1.0.0.tar.zst
        accept: */*
        accept-encoding: gzip, br, deflate
        host: ...
        user-agent: ...

        200 OK
        accept-ranges: bytes
        content-length: ...
        content-type: application/octet-stream
        etag: ...
        last-modified: ...
    "]];
    expected.assert_eq(&registry.logs());
}

#[test]
fn not_found() {
    let mut registry = HttpRegistry::serve();
    registry.publish(|t| {
        // Publish a package so that the directory hierarchy is created.
        // Note, however, that we declare a dependency on baZ.
        ProjectBuilder::start()
            .name("bar")
            .version("1.0.0")
            .lib_cairo(r#"fn f() -> felt252 { 0 }"#)
            .build(t);
    });

    let t = TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("foo")
        .version("0.1.0")
        .dep("baz", Dep.version("1").registry(&registry))
        .build(&t);

    Scarb::quick_snapbox()
        .arg("fetch")
        .current_dir(&t)
        .timeout(Duration::from_secs(10))
        .assert()
        .failure()
        .stdout_matches(indoc! {r#"
        error: failed to lookup for `baz ^1 (registry+http://[..])` in registry: registry+http://[..]

        Caused by:
            package not found in registry: baz ^1 (registry+http://[..])
        "#});

    let expected = expect![["
        GET /config.json
        accept: */*
        accept-encoding: gzip, br, deflate
        host: ...
        user-agent: ...

        200 OK
        accept-ranges: bytes
        content-length: ...
        content-type: application/json
        etag: ...
        last-modified: ...

        ###

        GET /index/3/b/baz.json
        accept: */*
        accept-encoding: gzip, br, deflate
        host: ...
        user-agent: ...

        404 Not Found
        content-length: 0
        etag: ...
    "]];
    expected.assert_eq(&registry.logs());
}

#[test]
fn missing_config_json() {
    let registry = HttpRegistry::serve();
    fs::remove_file(registry.child("config.json")).unwrap();

    let t = TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("foo")
        .version("0.1.0")
        .dep("baz", Dep.version("1").registry(&registry))
        .build(&t);

    Scarb::quick_snapbox()
        .arg("fetch")
        .current_dir(&t)
        .timeout(Duration::from_secs(10))
        .assert()
        .failure()
        .stdout_matches(indoc! {r#"
        error: failed to lookup for `baz ^1 (registry+http://[..])` in registry: registry+http://[..]

        Caused by:
            0: failed to fetch registry config
            1: HTTP status client error (404 Not Found) for url (http://[..]/config.json)
        "#});

    let expected = expect![["
        GET /config.json
        accept: */*
        accept-encoding: gzip, br, deflate
        host: ...
        user-agent: ...

        404 Not Found
        content-length: 0
        etag: ...
    "]];
    expected.assert_eq(&registry.logs());
}

#[test]
fn caching() {
    let cache_dir = TempDir::new().unwrap();

    let mut registry = HttpRegistry::serve();
    registry.publish(|t| {
        ProjectBuilder::start()
            .name("bar")
            .version("1.0.0")
            .lib_cairo(r#"fn f() -> felt252 { 0 }"#)
            .build(t);
    });

    let t = TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("foo")
        .version("0.1.0")
        .dep("bar", Dep.version("1").registry(&registry))
        .lib_cairo(r#"fn f() -> felt252 { bar::f() }"#)
        .build(&t);

    // FIXME(mkaput): Why are verbose statuses not appearing here?
    Scarb::quick_snapbox()
        .arg("fetch")
        .env("SCARB_CACHE", cache_dir.path())
        .current_dir(&t)
        .timeout(Duration::from_secs(10))
        .assert()
        .success()
        .stdout_matches(indoc! {r#"
        [..] Downloading bar v1.0.0 ([..])
        "#});

    // FIXME(mkaput): Why are verbose statuses not appearing here?
    Scarb::quick_snapbox()
        .arg("fetch")
        .env("SCARB_CACHE", cache_dir.path())
        .current_dir(&t)
        .timeout(Duration::from_secs(10))
        .assert()
        .success()
        .stdout_eq("");

    let expected = expect![[r#"
        GET /config.json
        accept: */*
        accept-encoding: gzip, br, deflate
        host: ...
        user-agent: ...

        200 OK
        accept-ranges: bytes
        content-length: ...
        content-type: application/json
        etag: ...
        last-modified: ...

        ###

        GET /index/3/b/bar.json
        accept: */*
        accept-encoding: gzip, br, deflate
        host: ...
        user-agent: ...

        200 OK
        accept-ranges: bytes
        content-length: ...
        content-type: application/json
        etag: ...
        last-modified: ...

        ###

        GET /bar-1.0.0.tar.zst
        accept: */*
        accept-encoding: gzip, br, deflate
        host: ...
        user-agent: ...

        200 OK
        accept-ranges: bytes
        content-length: ...
        content-type: application/octet-stream
        etag: ...
        last-modified: ...

        ###

        GET /config.json
        accept: */*
        accept-encoding: gzip, br, deflate
        host: ...
        user-agent: ...

        200 OK
        accept-ranges: bytes
        content-length: ...
        content-type: application/json
        etag: ...
        last-modified: ...

        ###

        GET /index/3/b/bar.json
        accept: */*
        accept-encoding: gzip, br, deflate
        host: ...
        if-none-match: ...
        user-agent: ...

        304 Not Modified
        content-length: 0
        etag: ...
    "#]];
    expected.assert_eq(&registry.logs());
}

// TODO(mkaput): Test errors properly when package is in index, but tarball is missing.
// TODO(mkaput): Test interdependencies.
// TODO(mkaput): Test offline mode, including with some cache prepopulated.
