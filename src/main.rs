use std::convert::Infallible;
use std::net::SocketAddr;
use hyper::{Body, Method, Request, Response, Server, StatusCode};
use hyper::service::{make_service_fn, service_fn};

async fn router(req: Request<Body>) -> Result<Response<Body>, Infallible> {
    match (req.method(), req.uri().path()) {
        (&Method::GET, "/health") => health_check().await,
        (&Method::POST, "/ai") => handle_ai_request(req).await,
        _ => not_found().await,
    }
}

async fn health_check() -> Result<Response<Body>, Infallible> {
    Ok(Response::new(Body::from("OK")))
}

async fn handle_ai_request(_req: Request<Body>) -> Result<Response<Body>, Infallible> {
    // TODO: Implement AI request handling logic
    Ok(Response::new(Body::from("AI request received")))
}

async fn not_found() -> Result<Response<Body>, Infallible> {
    let mut not_found = Response::default();
    *not_found.status_mut() = StatusCode::NOT_FOUND;
    Ok(not_found)
}

#[tokio::main]
async fn main() {
    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));

    let make_svc = make_service_fn(|_conn| async {
        Ok::<_, Infallible>(service_fn(router))
    });

    let server = Server::bind(&addr).serve(make_svc);

    println!("The Roost is running on http://{}", addr);

    if let Err(e) = server.await {
        eprintln!("server error: {}", e);
    }
}