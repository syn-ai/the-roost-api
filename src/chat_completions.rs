use hyper::{Body, Request, Response, StatusCode};
use serde::{Deserialize, Serialize};
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE};
use futures::StreamExt;
use bytes::BytesMut;
use crate::config::Settings;
#[derive(Debug, Deserialize, Serialize)]
pub struct ChatCompletionRequest {
    pub model: String,
    pub messages: Vec<Message>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Message {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ChatCompletionResponse {
    pub id: String,
    pub object: String,
    pub created: u64,
    pub model: String,
    pub choices: Vec<Choice>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Choice {
    pub index: u32,
    pub message: Message,
    pub finish_reason: String,
}

pub async fn handle_chat_completion(req: Request<Body>, settings: Settings) -> Result<Response<Body>, hyper::Error> {
    let mut body = BytesMut::new();
    let mut stream = req.into_body();

    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        body.extend_from_slice(&chunk);
        
        if body.len() > 10_000_000 {
            return Ok(Response::builder()
                .status(StatusCode::PAYLOAD_TOO_LARGE)
                .body(Body::from("Request body too large"))
                .unwrap());
        }
    }

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
    headers.insert(AUTHORIZATION, HeaderValue::from_str(&format!("Bearer {}", settings.ai_service.api_key)).unwrap());
    headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));

    let response = client.post(settings.ai_service.url + "/v1/chat/completions")
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