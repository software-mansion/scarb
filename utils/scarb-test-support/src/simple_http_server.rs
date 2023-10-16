use std::net::{SocketAddr, TcpListener};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::Arc;

use axum::body::Body;
use axum::extract::State;
use axum::http::header::ETAG;
use axum::http::Request;
use axum::middleware::Next;
use axum::response::Response;
use axum::{middleware, Router};
use data_encoding::HEXLOWER;
use itertools::Itertools;
use sha2::digest::FixedOutput;
use sha2::Digest;
use tower_http::services::ServeDir;

pub struct SimpleHttpServer {
    addr: SocketAddr,
    log_requests: Arc<AtomicBool>,
    ct: Option<tokio::sync::oneshot::Sender<()>>,
}

impl SimpleHttpServer {
    pub fn serve(dir: PathBuf) -> Self {
        let (ct, ctrx) = tokio::sync::oneshot::channel::<()>();

        let log_requests = Arc::new(AtomicBool::new(false));

        let app = Router::new()
            .fallback_service(ServeDir::new(dir))
            .layer(middleware::from_fn_with_state(log_requests.clone(), logger))
            .layer(middleware::map_response(set_etag));

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
            log_requests,
            ct: Some(ct),
        }
    }

    pub fn url(&self) -> String {
        format!("http://{}/", self.addr)
    }

    /// Enable this when writing tests to see what requests are being made in the test.
    pub fn log_requests(&self, enable: bool) {
        self.log_requests.store(enable, Ordering::Relaxed);
    }
}

impl Drop for SimpleHttpServer {
    fn drop(&mut self) {
        let _ = self.ct.take().map(|ct| ct.send(()));
    }
}

async fn logger<B>(
    State(enable): State<Arc<AtomicBool>>,
    request: Request<B>,
    next: Next<B>,
) -> Response {
    static COUNTER: AtomicU32 = AtomicU32::new(0);
    let count = COUNTER.fetch_add(1, Ordering::Relaxed);

    let enable = enable.load(Ordering::Relaxed);

    if enable {
        eprintln!(
            "http[{count}]: {method} {uri}",
            method = request.method(),
            uri = request.uri()
        );
    }

    let response = next.run(request).await;

    if enable {
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

    response
}

async fn set_etag(res: Response) -> Response<Body> {
    let (mut parts, body) = res.into_parts();
    let bytes = hyper::body::to_bytes(body).await.unwrap();

    let mut digest = sha2::Sha256::new();
    digest.update(&bytes);
    let digest: [u8; 32] = digest.finalize_fixed().into();
    let digest = HEXLOWER.encode(&digest);

    parts.headers.insert(ETAG, digest.parse().unwrap());

    Response::from_parts(parts, Body::from(bytes))
}
