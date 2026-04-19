use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde_json::json;

#[derive(Debug)]
pub enum ApiError {
    GameNotFound,
    BadMove(String),
    GameFinished,
    BadRequest(String),
    NoLegalGreedyMove,
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, msg) = match self {
            ApiError::GameNotFound => (StatusCode::NOT_FOUND, "game not found".to_string()),
            ApiError::BadMove(s) => (StatusCode::BAD_REQUEST, format!("illegal move: {s}")),
            ApiError::GameFinished => (StatusCode::CONFLICT, "game already finished".to_string()),
            ApiError::BadRequest(s) => (StatusCode::BAD_REQUEST, s),
            ApiError::NoLegalGreedyMove => {
                (StatusCode::CONFLICT, "no legal greedy move available".to_string())
            }
        };
        (status, Json(json!({ "error": msg }))).into_response()
    }
}
