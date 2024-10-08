mod chat_completions;
mod config;

use std::convert::Infallible;
use std::net::SocketAddr;
use hyper::{Body, Method, Request, Response, Server, StatusCode};
use hyper::service::{make_service_fn, service_fn};
use crate::config::Settings;

async fn router(req: Request<Body>, settings: Settings) -> Result<Response<Body>, hyper::Error> {
    println!("Received request: {} {}", req.method(), req.uri().path());
    match (req.method(), req.uri().path()) {
        (&Method::POST, "/v1/chat/completions") => chat_completions::handle_chat_completion(req, settings).await,
        (&Method::GET, "/health") => health_check(req).await,
        _ => {
            println!("No route matched, returning 404");
            not_found().await
        }
    }
}

async fn health_check(_req: Request<Body>) -> Result<Response<Body>, hyper::Error> {
    // Return an error response for the health check
    Ok(Response::builder()
        .status(StatusCode::INTERNAL_SERVER_ERROR)
        .body(Body::from("Service Unhealthy"))
        .unwrap())
}

async fn not_found() -> Result<Response<Body>, hyper::Error> {
    let mut not_found = Response::default();
    *not_found.status_mut() = StatusCode::NOT_FOUND;
    Ok(not_found)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let settings = Settings::new()?;

    let addr: SocketAddr = format!("{}:{}", settings.server.host, settings.server.port)
        .parse()
        .expect("Failed to parse address");

    let make_svc = make_service_fn(|_conn| {
        let settings = settings.clone();
        async move {
            Ok::<_, Infallible>(service_fn(move |req| {
                let settings = settings.clone();
                router(req, settings)
            }))
        }
    });

    let server = Server::bind(&addr).serve(make_svc);

    println!("The Roost is running on http://{}", addr);

    server.await?;

    Ok(())
}