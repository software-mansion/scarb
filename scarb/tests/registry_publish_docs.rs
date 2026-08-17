use std::time::Duration;

use assert_fs::TempDir;
use expect_test::expect;
use indoc::indoc;

use scarb_test_support::command::Scarb;
use scarb_test_support::project_builder::ProjectBuilder;
use scarb_test_support::registry::http::HttpRegistry;
use scarb_test_support::simple_http_server::HttpPostResponse;

#[test]
fn publish_docs() {
    // 200 -> StatusCode::OK
    let registry = HttpRegistry::serve(Some(HttpPostResponse {
        code: 200,
        message: Some("published".to_string()),
    }));

    let t = TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("bar")
        .version("1.0.0")
        .lib_cairo(r#"fn f() -> felt252 { 0 }"#)
        .build(&t);

    Scarb::quick_command()
        .arg("publish-docs")
        .arg("--index")
        .arg(&registry.url)
        .env("SCARB_REGISTRY_AUTH_TOKEN", "scrb_supersecrettoken")
        .current_dir(&t)
        .timeout(Duration::from_secs(60))
        .assert()
        .success()
        .stdout_eq(indoc! {r#"
        [..] Packaged [..]
        [..] Uploading docs for bar v1.0.0 (registry+http[..])
        [..] Published docs for bar v1.0.0 (registry+http[..])
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

    POST /api/v1/docs/bar/1.0.0
    accept: */*
    accept-encoding: gzip, br, deflate
    authorization: Bearer scrb_supersecrettoken
    content-type: ...
    host: ...
    transfer-encoding: chunked
    user-agent: ...

    200 OK
    content-type: text/plain; charset=utf-8
    etag: ...
    "]];
    expected.assert_eq(&registry.logs());
}

#[test]
fn auth_token_missing() {
    // 200 -> StatusCode::OK
    let registry = HttpRegistry::serve(Some(HttpPostResponse {
        code: 200,
        message: Some("missing authentication token. help: make sure SCARB_REGISTRY_AUTH_TOKEN environment variable is set"
                    .to_string())
    }));

    let t = TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("bar")
        .version("1.0.0")
        .lib_cairo(r#"fn f() -> felt252 { 0 }"#)
        .build(&t);

    Scarb::quick_command()
        .arg("publish-docs")
        .arg("--index")
        .arg(&registry.url)
        .current_dir(&t)
        .timeout(Duration::from_secs(60))
        .assert()
        .failure()
        .stdout_eq(indoc! {r#"
        [..] Packaged [..]
        [..] Uploading docs for bar v1.0.0 (registry+http[..])
        error: missing authentication token. help: make sure SCARB_REGISTRY_AUTH_TOKEN environment variable is set
        "#});
}

#[test]
fn error_from_registry() {
    // 400 -> StatusCode::BAD_REQUEST
    let registry = HttpRegistry::serve(Some(HttpPostResponse {
        code: 409,
        message: Some("Docs for 'bar' already exist. Use force to overwrite.".to_string()),
    }));

    let t = TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("bar")
        .version("1.0.0")
        .lib_cairo(r#"fn f() -> felt252 { 0 }"#)
        .build(&t);

    Scarb::quick_command()
        .arg("publish-docs")
        .arg("--index")
        .arg(&registry.url)
        .env("SCARB_REGISTRY_AUTH_TOKEN", "scrb_supersecrettoken")
        .current_dir(&t)
        .timeout(Duration::from_secs(60))
        .assert()
        .failure()
        .stdout_eq(indoc! {r#"
        [..] Packaged [..]
        [..] Uploading docs for bar v1.0.0 (registry+http[..])
        error: upload failed with status code: `409 Conflict`, `Docs for 'bar' already exist. Use force to overwrite.`
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

    POST /api/v1/docs/bar/1.0.0
    accept: */*
    accept-encoding: gzip, br, deflate
    authorization: Bearer scrb_supersecrettoken
    content-type: ...
    host: ...
    transfer-encoding: chunked
    user-agent: ...

    409 Conflict
    content-type: text/plain; charset=utf-8
    etag: ...
    "]];
    expected.assert_eq(&registry.logs());
}
