use std::convert::Infallible;
use hyper::{Body, Method, Request, Response, Server, StatusCode};
use hyper::service::{make_service_fn, service_fn};
use serde::{Deserialize, Serialize};
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE};
use futures::StreamExt;
use bytes::BytesMut;

#[derive(Debug, Deserialize, Serialize)]
struct ChatCompletionRequest {
    model: String,
    messages: Vec<Message>,
}

#[derive(Debug, Deserialize, Serialize)]
struct Message {
    role: String,
    content: String,
}

#[derive(Debug, Deserialize, Serialize)]
struct ChatCompletionResponse {
    id: String,
    object: String,
    created: u64,
    model: String,
    choices: Vec<Choice>,
}

#[derive(Debug, Deserialize, Serialize)]
struct Choice {
    index: u32,
    message: Message,
    finish_reason: String,
}

async fn router(req: Request<Body>) -> Result<Response<Body>, Infallible> {
    println!("Received request: {} {}", req.method(), req.uri().path());
    match (req.method(), req.uri().path()) {
        (&Method::POST, "/v1/chat/completions") => handle_chat_completion(req).await,
        (&Method::GET, "/health") => health_check().await,
        _ => {
            println!("No route matched, returning 404");
            not_found().await
        }
    }
}

async fn handle_chat_completion(req: Request<Body>) -> Result<Response<Body>, Infallible> {
    let mut body = BytesMut::new();
    let mut stream = req.into_body();

    // Stream the body chunks
    while let Some(chunk) = stream.next().await {
        let chunk = match chunk {
            Ok(chunk) => chunk,
            Err(e) => {
                eprintln!("Failed to read chunk: {:?}", e);
                return Ok(Response::builder()
                    .status(StatusCode::BAD_REQUEST)
                    .body(Body::from("Failed to read request body"))
                    .unwrap());
            }
        };
        body.extend_from_slice(&chunk);
        
        // Optional: Check if body exceeds a maximum size (e.g., 10 MB)
        if body.len() > 10_000_000 {
            return Ok(Response::builder()
                .status(StatusCode::PAYLOAD_TOO_LARGE)
                .body(Body::from("Request body too large"))
                .unwrap());
        }
    }

    // Deserialize the request
    let chat_req: ChatCompletionRequest = match serde_json::from_slice(&body) {
        Ok(req) => req,
        Err(e) => {
            eprintln!("Failed to parse request: {:?}", e);
            return Ok(Response::builder()
                .status(StatusCode::BAD_REQUEST)
                .body(Body::from("Invalid request format"))
                .unwrap());
        }
    };

    let client = reqwest::Client::new();
    let mut headers = HeaderMap::new();
    headers.insert(AUTHORIZATION, HeaderValue::from_static("Bearer sk-1234"));
    headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));

    // Send request to AI service
    let response = client.post("https://hub-agentartificial.ngrok.dev/v1/api/chat/completions")
        .headers(headers)
        .json(&chat_req)
        .timeout(std::time::Duration::from_secs(30))
        .send()
        .await;

    match response {
        Ok(res) => {
            if res.status().is_success() {
                match res.json::<ChatCompletionResponse>().await {
                    Ok(chat_response) => {
                        let json = serde_json::to_string(&chat_response).unwrap();
                        Ok(Response::builder()
                            .status(StatusCode::OK)
                            .header(CONTENT_TYPE, "application/json")
                            .body(Body::from(json))
                            .unwrap())
                    },
                    Err(e) => {
                        eprintln!("Failed to parse AI service response: {:?}", e);
                        Ok(Response::builder()
                            .status(StatusCode::INTERNAL_SERVER_ERROR)
                            .body(Body::from("Failed to process AI service response"))
                            .unwrap())
                    }
                }
            } else {
                let error_body = format!("AI service error: {}", res.status());
                Ok(Response::builder()
                    .status(res.status())
                    .body(Body::from(error_body))
                    .unwrap())
            }
        },
        Err(e) => {
            eprintln!("Failed to send request to AI service: {:?}", e);
            Ok(Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(Body::from("Failed to communicate with AI service"))
                .unwrap())
        }
    }
}

async fn health_check() -> Result<Response<Body>, Infallible> {
    Ok(Response::new(Body::from("OK")))
}

async fn not_found() -> Result<Response<Body>, Infallible> {
    let mut not_found = Response::default();
    *not_found.status_mut() = StatusCode::NOT_FOUND;
    Ok(not_found)
}



#[tokio::main]
async fn main() {
    let addr = ([127, 0, 0, 1], 3000).into();

    let make_svc = make_service_fn(|_conn| async {
        Ok::<_, Infallible>(service_fn(router))
    });

    let server = Server::bind(&addr).serve(make_svc);

    println!("The Roost is running on http://{}", addr);

    if let Err(e) = server.await {
        eprintln!("server error: {}", e);
    }
}
