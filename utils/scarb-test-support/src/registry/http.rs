use std::fmt;
use std::path::Path;

use assert_fs::fixture::ChildPath;
use assert_fs::prelude::*;
use assert_fs::TempDir;
use once_cell::sync::Lazy;
use serde_json::json;
use tokio::runtime;

use crate::registry::local::LocalRegistry;
use crate::simple_http_server::SimpleHttpServer;

// Keep a global multi-threading runtime to contain all running servers in one shared
// thread pool, while maintaining synchronous nature of tests.
static RUNTIME: Lazy<runtime::Runtime> = Lazy::new(|| {
    runtime::Builder::new_multi_thread()
        .worker_threads(1)
        .enable_all()
        .build()
        .unwrap()
});

pub struct HttpRegistry {
    local: LocalRegistry,
    url: String,

    // This needs to be stored here so that it's dropped properly.
    server: SimpleHttpServer,
}

impl HttpRegistry {
    pub fn serve() -> Self {
        let local = LocalRegistry::create();
        let server = {
            let _guard = RUNTIME.enter();
            SimpleHttpServer::serve(local.t.path().to_owned())
        };
        let url = server.url();

        let config = json!({
            "version": 1,
            "dl": format!("{url}{{package}}-{{version}}.tar.zst"),
            "index": format!("{url}index/{{prefix}}/{{package}}.json")
        });
        local
            .t
            .child("config.json")
            .write_str(&serde_json::to_string(&config).unwrap())
            .unwrap();

        Self { local, url, server }
    }

    pub fn publish(&mut self, f: impl FnOnce(&TempDir)) -> &mut Self {
        self.local.publish(f);
        self
    }

    /// Enable this when writing tests to see what requests are being made in the test.
    pub fn print_logs(&self) {
        self.server.print_logs(true);
    }

    pub fn logs(&self) -> String {
        let _guard = RUNTIME.enter();
        RUNTIME.block_on(async { self.server.logs_to_string().await })
    }
}

impl PathChild for HttpRegistry {
    fn child<P>(&self, path: P) -> ChildPath
    where
        P: AsRef<Path>,
    {
        self.local.t.child(path)
    }
}

impl fmt::Display for HttpRegistry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.url, f)
    }
}
