use std::time::Duration;

use assert_fs::TempDir;
use expect_test::expect;
use indoc::indoc;

use scarb_test_support::command::Scarb;
use scarb_test_support::project_builder::ProjectBuilder;
use scarb_test_support::registry::http::HttpRegistry;
use scarb_test_support::simple_http_server::HttpPostResponse;

#[test]
fn publish() {
    // 200 -> StatusCode::OK
    let registry = HttpRegistry::serve(Some(HttpPostResponse {
        code: 200,
        message: "published".to_string(),
    }));

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
    authorization: Bearer scrb_supersecrettoken
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
    let registry = HttpRegistry::serve(
        Some(
            HttpPostResponse {
                code: 200,
                message: "missing authentication token. help: make sure SCARB_REGISTRY_AUTH_TOKEN environment variable is set"
                    .to_string()
            }
        ));

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
        error: missing authentication token. help: make sure SCARB_REGISTRY_AUTH_TOKEN environment variable is set
        "#});
}

#[test]
fn error_from_registry() {
    // 400 -> StatusCode::BAD_REQUEST
    let registry = HttpRegistry::serve(Some(HttpPostResponse {
        code: 400,
        message: "Version '1.0.0' of package 'bar' already exists.".to_string(),
    }));

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
        error: upload failed with status code: `400 Bad Request`, `Version '1.0.0' of package 'bar' already exists.`
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
    authorization: Bearer scrb_supersecrettoken
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
