use std::collections::BTreeMap;
use std::fmt;
use std::net::{SocketAddr, TcpListener};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::Arc;

use axum::body::Body;
use axum::extract::State;
use axum::http::header::{ETAG, IF_MODIFIED_SINCE, IF_NONE_MATCH};
use axum::http::Method;
use axum::http::Request;
use axum::http::StatusCode;
use axum::http::{HeaderMap, HeaderValue};
use axum::middleware;
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use axum::routing::post;
use axum::Router;
use data_encoding::HEXLOWER;
use itertools::Itertools;
use sha2::digest::FixedOutput;
use sha2::Digest;
use tokio::sync::Mutex;
use tower_http::services::ServeDir;

/// Key is request count from logger middleware. Keeping logs in ordered map allows sorting logs by
/// request incoming time, not by response outgoing one.
type LogsStore = Arc<Mutex<BTreeMap<u32, HttpLog>>>;

type LoggerState = (LogsStore, Arc<AtomicBool>);

pub struct SimpleHttpServer {
    addr: SocketAddr,
    print_logs: Arc<AtomicBool>,
    logs: LogsStore,
    ct: Option<tokio::sync::oneshot::Sender<()>>,
}

pub struct HttpLog {
    pub req_method: Method,
    pub req_uri: String,
    pub req_headers: HeaderMap,
    pub res_status: StatusCode,
    pub res_headers: HeaderMap,
}

impl SimpleHttpServer {
    pub fn serve(dir: PathBuf, post_status: Option<u16>) -> Self {
        let (ct, ctrx) = tokio::sync::oneshot::channel::<()>();

        let print_logs = Arc::new(AtomicBool::new(false));
        let logs: LogsStore = Default::default();

        let app = Router::new()
            .fallback_service(ServeDir::new(dir))
            .route(
                "/api/v1/packages/new",
                post(move || post_handler(post_status)),
            )
            .layer(middleware::from_fn(set_etag))
            .layer(middleware::from_fn_with_state(
                (logs.clone(), print_logs.clone()),
                logger,
            ));

        let tcp = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = tcp.local_addr().unwrap();
        let server = axum::Server::from_tcp(tcp)
            .unwrap()
            .serve(app.into_make_service());

        tokio::spawn(async move {
            let graceful = server.with_graceful_shutdown(async {
                ctrx.await.ok();
            });

            let _ = graceful.await;
        });

        Self {
            addr,
            print_logs,
            logs,
            ct: Some(ct),
        }
    }

    pub fn url(&self) -> String {
        format!("http://{}/", self.addr)
    }

    /// Enable this when writing tests to see what requests are being made in the test.
    pub fn print_logs(&self, enable: bool) {
        self.print_logs.store(enable, Ordering::Relaxed);
    }

    pub async fn logs_to_string(&self) -> String {
        let logs = self.logs.lock().await;
        logs.values().map(ToString::to_string).join("\n###\n\n")
    }
}

impl Drop for SimpleHttpServer {
    fn drop(&mut self) {
        let _ = self.ct.take().map(|ct| ct.send(()));
    }
}

async fn post_handler(post_status: Option<u16>) -> impl IntoResponse {
    let status_code = post_status
        .and_then(|code| StatusCode::from_u16(code).ok())
        .unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
    (status_code, "POST request received")
}

async fn logger<B>(
    State((logs, print_logs)): State<LoggerState>,
    request: Request<B>,
    next: Next<B>,
) -> Response {
    static COUNTER: AtomicU32 = AtomicU32::new(0);
    let count = COUNTER.fetch_add(1, Ordering::Relaxed);

    let print_logs = print_logs.load(Ordering::Relaxed);

    if print_logs {
        eprintln!(
            "http[{count}]: {method} {uri}",
            method = request.method(),
            uri = request.uri()
        );
    }

    let req_method = request.method().clone();
    let req_uri = request.uri().to_string();
    let req_headers = request.headers().clone();

    let response = next.run(request).await;

    if print_logs {
        eprintln!(
            "http[{count}]: {status} => {headers}",
            count = count,
            status = response.status(),
            headers = response
                .headers()
                .iter()
                .sorted_by_key(|(k, _)| k.as_str())
                .map(|(k, v)| format!("{k}: {v}", v = v.to_str().unwrap_or("<bytes>")))
                .join(", "),
        );
    }

    let res_status = response.status();
    let res_headers = response.headers().clone();

    let log = HttpLog {
        req_method,
        req_uri,
        req_headers,
        res_status,
        res_headers,
    };

    {
        let mut logs = logs.lock().await;
        logs.insert(count, log);
    }

    response
}

async fn set_etag<B>(request: Request<B>, next: Next<B>) -> Response<Body> {
    let if_none_match = request.headers().get(IF_NONE_MATCH).cloned();

    if if_none_match.is_none() && request.headers().contains_key(IF_MODIFIED_SINCE) {
        todo!("This server does not support If-Modified-Since header.")
    }

    let res = next.run(request).await;

    let (mut parts, body) = res.into_parts();
    let bytes = hyper::body::to_bytes(body).await.unwrap();

    let mut digest = sha2::Sha256::new();
    digest.update(&bytes);
    let digest: [u8; 32] = digest.finalize_fixed().into();
    let digest = HEXLOWER.encode(&digest);
    let digest = HeaderValue::from_str(&digest).unwrap();

    if let Some(if_none_match) = if_none_match {
        if digest == if_none_match {
            parts.status = StatusCode::NOT_MODIFIED;
            parts.headers = HeaderMap::from_iter([(ETAG, digest)]);
            return Response::from_parts(parts, Body::empty());
        }
    }

    parts.headers.insert(ETAG, digest);
    Response::from_parts(parts, Body::from(bytes))
}

impl fmt::Display for HttpLog {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use axum::http::header::*;

        writeln!(
            f,
            "{method} {uri}",
            method = self.req_method,
            uri = self.req_uri
        )?;
        self.req_headers
            .iter()
            .sorted_by_key(|(k, _)| k.as_str())
            .map(|(k, v)| (k, String::from_utf8_lossy(v.as_bytes())))
            .map(|(k, v)| match *k {
                HOST | IF_NONE_MATCH | IF_MODIFIED_SINCE | USER_AGENT | CONTENT_TYPE => {
                    (k, "...".into())
                }
                _ => (k, v),
            })
            .try_for_each(|(k, v)| writeln!(f, "{k}: {v}"))?;
        writeln!(f)?;
        writeln!(f, "{status}", status = self.res_status)?;
        self.res_headers
            .iter()
            .sorted_by_key(|(k, _)| k.as_str())
            .map(|(k, v)| (k, String::from_utf8_lossy(v.as_bytes())))
            .map(|(k, v)| match *k {
                CONTENT_LENGTH if v == "0" => (k, v),
                CONTENT_LENGTH => (k, "...".into()),
                ETAG | LAST_MODIFIED => (k, "...".into()),
                _ => (k, v),
            })
            .try_for_each(|(k, v)| writeln!(f, "{k}: {v}"))?;
        Ok(())
    }
}
