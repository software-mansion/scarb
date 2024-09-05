use std::time::Duration;

use assert_fs::TempDir;
use expect_test::expect;
use indoc::indoc;

use scarb_test_support::command::Scarb;
use scarb_test_support::project_builder::ProjectBuilder;
use scarb_test_support::registry::http::HttpRegistry;

#[test]
fn publish() {
    // 200 -> StatusCode::OK
    let registry = HttpRegistry::serve(Some(200));

    let t = TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("bar")
        .version("1.0.0")
        .lib_cairo(r#"fn f() -> felt252 { 0 }"#)
        .build(&t);

    Scarb::quick_snapbox()
        .arg("publish")
        .arg("--index")
        .arg(&registry.url)
        .arg("--no-verify")
        .env("SCARB_REGISTRY_AUTH_TOKEN", "scrb_supersecrettoken")
        .current_dir(&t)
        .timeout(Duration::from_secs(60))
        .assert()
        .success()
        .stdout_matches(indoc! {r#"
        [..] Packaging bar v1.0.0 ([..])
        warn: manifest has no readme
        warn: manifest has no description
        warn: manifest has no license or license-file
        warn: manifest has no documentation or homepage or repository
        see [..]
        [..]
        [..] Packaged [..]
        [..] Uploading bar v1.0.0 (registry+http[..])
        [..] Published bar v1.0.0 (registry+http[..])
        "#});

    let expected = expect![["
    GET /api/v1/index/config.json
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

    POST /api/v1/packages/new
    accept: */*
    accept-encoding: gzip, br, deflate
    authorization: scrb_supersecrettoken
    content-type: ...
    host: ...
    transfer-encoding: chunked
    user-agent: ...

    200 OK
    content-length: ...
    content-type: text/plain; charset=utf-8
    etag: ...
    "]];
    expected.assert_eq(&registry.logs());
}

#[test]
fn auth_token_missing() {
    // 200 -> StatusCode::OK
    let registry = HttpRegistry::serve(Some(200));

    let t = TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("bar")
        .version("1.0.0")
        .lib_cairo(r#"fn f() -> felt252 { 0 }"#)
        .build(&t);

    Scarb::quick_snapbox()
        .arg("publish")
        .arg("--index")
        .arg(&registry.url)
        .arg("--no-verify")
        .current_dir(&t)
        .timeout(Duration::from_secs(60))
        .assert()
        .failure()
        .stdout_matches(indoc! {r#"
        [..] Packaging bar v1.0.0 ([..])
        warn: manifest has no readme
        warn: manifest has no description
        warn: manifest has no license or license-file
        warn: manifest has no documentation or homepage or repository
        see [..]
        [..]
        [..] Packaged [..]
        [..] Uploading bar v1.0.0 (registry+http[..])
        error: missing authentication token
        "#});
}

#[test]
fn invalid_auth_token() {
    // 401 -> StatusCode::UNAUTHORIZED
    let registry = HttpRegistry::serve(Some(401));

    let t = TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("bar")
        .version("1.0.0")
        .lib_cairo(r#"fn f() -> felt252 { 0 }"#)
        .build(&t);

    Scarb::quick_snapbox()
        .arg("publish")
        .arg("--index")
        .arg(&registry.url)
        .arg("--no-verify")
        .env("SCARB_REGISTRY_AUTH_TOKEN", "scrb_supersecrettoken")
        .current_dir(&t)
        .timeout(Duration::from_secs(60))
        .assert()
        .failure()
        .stdout_matches(indoc! {r#"
        [..] Packaging bar v1.0.0 ([..])
        warn: manifest has no readme
        warn: manifest has no description
        warn: manifest has no license or license-file
        warn: manifest has no documentation or homepage or repository
        see [..]
        [..]
        [..] Packaged [..]
        [..] Uploading bar v1.0.0 (registry+http[..])
        error: invalid authentication token
        "#});

    let expected = expect![["
    GET /api/v1/index/config.json
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

    POST /api/v1/packages/new
    accept: */*
    accept-encoding: gzip, br, deflate
    authorization: scrb_supersecrettoken
    content-type: ...
    host: ...
    transfer-encoding: chunked
    user-agent: ...

    401 Unauthorized
    content-length: ...
    content-type: text/plain; charset=utf-8
    etag: ...
    "]];
    expected.assert_eq(&registry.logs());
}

#[test]
fn missing_upload_permission() {
    // 403 -> StatusCode::FORBIDDEN
    let registry = HttpRegistry::serve(Some(403));

    let t = TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("bar")
        .version("1.0.0")
        .lib_cairo(r#"fn f() -> felt252 { 0 }"#)
        .build(&t);

    Scarb::quick_snapbox()
        .arg("publish")
        .arg("--index")
        .arg(&registry.url)
        .arg("--no-verify")
        .env("SCARB_REGISTRY_AUTH_TOKEN", "scrb_supersecrettoken")
        .current_dir(&t)
        .timeout(Duration::from_secs(60))
        .assert()
        .failure()
        .stdout_matches(indoc! {r#"
        [..] Packaging bar v1.0.0 ([..])
        warn: manifest has no readme
        warn: manifest has no description
        warn: manifest has no license or license-file
        warn: manifest has no documentation or homepage or repository
        see [..]
        [..]
        [..] Packaged [..]
        [..] Uploading bar v1.0.0 (registry+http[..])
        error: missing upload permissions or not the package owner
        "#});

    let expected = expect![["
    GET /api/v1/index/config.json
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

    POST /api/v1/packages/new
    accept: */*
    accept-encoding: gzip, br, deflate
    authorization: scrb_supersecrettoken
    content-type: ...
    host: ...
    transfer-encoding: chunked
    user-agent: ...

    403 Forbidden
    content-length: ...
    content-type: text/plain; charset=utf-8
    etag: ...
    "]];
    expected.assert_eq(&registry.logs());
}

#[test]
fn version_exists() {
    // 400 -> StatusCode::BAD_REQUEST
    let registry = HttpRegistry::serve(Some(400));

    let t = TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("bar")
        .version("1.0.0")
        .lib_cairo(r#"fn f() -> felt252 { 0 }"#)
        .build(&t);

    Scarb::quick_snapbox()
        .arg("publish")
        .arg("--index")
        .arg(&registry.url)
        .arg("--no-verify")
        .env("SCARB_REGISTRY_AUTH_TOKEN", "scrb_supersecrettoken")
        .current_dir(&t)
        .timeout(Duration::from_secs(60))
        .assert()
        .failure()
        .stdout_matches(indoc! {r#"
        [..] Packaging bar v1.0.0 ([..])
        warn: manifest has no readme
        warn: manifest has no description
        warn: manifest has no license or license-file
        warn: manifest has no documentation or homepage or repository
        see [..]
        [..]
        [..] Packaged [..]
        [..] Uploading bar v1.0.0 (registry+http[..])
        error: package `bar v1.0.0 ([..])` already exists
        "#});

    let expected = expect![["
    GET /api/v1/index/config.json
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

    POST /api/v1/packages/new
    accept: */*
    accept-encoding: gzip, br, deflate
    authorization: scrb_supersecrettoken
    content-type: ...
    host: ...
    transfer-encoding: chunked
    user-agent: ...

    400 Bad Request
    content-length: ...
    content-type: text/plain; charset=utf-8
    etag: ...
    "]];
    expected.assert_eq(&registry.logs());
}

#[test]
fn file_corrupted() {
    // 422 -> StatusCode::UNPROCESSABLE_ENTITY
    let registry = HttpRegistry::serve(Some(422));

    let t = TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("bar")
        .version("1.0.0")
        .lib_cairo(r#"fn f() -> felt252 { 0 }"#)
        .build(&t);

    Scarb::quick_snapbox()
        .arg("publish")
        .arg("--index")
        .arg(&registry.url)
        .arg("--no-verify")
        .env("SCARB_REGISTRY_AUTH_TOKEN", "scrb_supersecrettoken")
        .current_dir(&t)
        .timeout(Duration::from_secs(60))
        .assert()
        .failure()
        .stdout_matches(indoc! {r#"
        [..] Packaging bar v1.0.0 ([..])
        warn: manifest has no readme
        warn: manifest has no description
        warn: manifest has no license or license-file
        warn: manifest has no documentation or homepage or repository
        see [..]
        [..]
        [..] Packaged [..]
        [..] Uploading bar v1.0.0 (registry+http[..])
        error: file corrupted during upload
        "#});

    let expected = expect![["
    GET /api/v1/index/config.json
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

    POST /api/v1/packages/new
    accept: */*
    accept-encoding: gzip, br, deflate
    authorization: scrb_supersecrettoken
    content-type: ...
    host: ...
    transfer-encoding: chunked
    user-agent: ...

    422 Unprocessable Entity
    content-length: ...
    content-type: text/plain; charset=utf-8
    etag: ...
    "]];
    expected.assert_eq(&registry.logs());
}

#[test]
fn unexpected_error() {
    // 501 -> StatusCode::NOT_IMPLEMENTED;
    // the code does not matter for this test
    // as long as it's not one of the cases handled by publish explicitly
    let registry = HttpRegistry::serve(Some(501));

    let t = TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("bar")
        .version("1.0.0")
        .lib_cairo(r#"fn f() -> felt252 { 0 }"#)
        .build(&t);

    Scarb::quick_snapbox()
        .arg("publish")
        .arg("--index")
        .arg(&registry.url)
        .arg("--no-verify")
        .env("SCARB_REGISTRY_AUTH_TOKEN", "scrb_supersecrettoken")
        .current_dir(&t)
        .timeout(Duration::from_secs(60))
        .assert()
        .failure()
        .stdout_matches(indoc! {r#"
        [..] Packaging bar v1.0.0 ([..])
        warn: manifest has no readme
        warn: manifest has no description
        warn: manifest has no license or license-file
        warn: manifest has no documentation or homepage or repository
        see [..]
        [..]
        [..] Packaged [..]
        [..] Uploading bar v1.0.0 (registry+http[..])
        error: upload failed with an unexpected error (trace-id: [..])
        "#});

    let expected = expect![["
    GET /api/v1/index/config.json
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

    POST /api/v1/packages/new
    accept: */*
    accept-encoding: gzip, br, deflate
    authorization: scrb_supersecrettoken
    content-type: ...
    host: ...
    transfer-encoding: chunked
    user-agent: ...

    501 Not Implemented
    content-length: ...
    content-type: text/plain; charset=utf-8
    etag: ...
    "]];
    expected.assert_eq(&registry.logs());
}
