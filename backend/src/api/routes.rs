use axum::extract::{Path, State};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::domain::feature::PlayerId;
use crate::domain::game::Game;
use crate::domain::greedy::{choose_move, GreedyMove};
use crate::domain::scoring::ScoringEvent;
use crate::domain::snapshot::GameView;

use super::error::ApiError;
use super::state::GameStore;

#[derive(Debug, Deserialize)]
pub struct CreateGameRequest {
    pub num_players: u8,
    pub seed: Option<u64>,
}

#[derive(Debug, Serialize)]
pub struct CreateGameResponse {
    pub game_id: Uuid,
    pub state: GameView,
}

#[derive(Debug, Serialize)]
pub struct TurnResponse {
    pub state: GameView,
    pub events: Vec<ScoringEvent>,
    pub finished: bool,
}

pub fn router(store: GameStore) -> Router {
    Router::new()
        .route("/games", post(create_game))
        .route("/games/{id}", get(get_game))
        .route("/games/{id}/turn", post(play_turn))
        .route("/games/{id}/bot-turn", post(bot_turn))
        .route("/games/{id}/legal-moves", get(legal_moves))
        .with_state(store)
}

async fn create_game(
    State(store): State<GameStore>,
    Json(req): Json<CreateGameRequest>,
) -> Result<Json<CreateGameResponse>, ApiError> {
    if !(1..=5).contains(&req.num_players) {
        return Err(ApiError::BadRequest("num_players must be in 1..=5".into()));
    }
    let seed = req.seed.unwrap_or_else(rand::random);
    let mut game = Game::new(req.num_players, seed);
    game.ensure_drawable();
    let id = Uuid::new_v4();
    let state = GameView::from_game(&game);
    store.lock().await.insert(id, game);
    Ok(Json(CreateGameResponse { game_id: id, state }))
}

async fn get_game(
    State(store): State<GameStore>,
    Path(id): Path<Uuid>,
) -> Result<Json<GameView>, ApiError> {
    let mut games = store.lock().await;
    let game = games.get_mut(&id).ok_or(ApiError::GameNotFound)?;
    game.ensure_drawable();
    Ok(Json(GameView::from_game(game)))
}

async fn play_turn(
    State(store): State<GameStore>,
    Path(id): Path<Uuid>,
    Json(mv): Json<GreedyMove>,
) -> Result<Json<TurnResponse>, ApiError> {
    let mut games = store.lock().await;
    let game = games.get_mut(&id).ok_or(ApiError::GameNotFound)?;
    let events = game
        .play_move(mv)
        .map_err(|e| ApiError::BadMove(format!("{:?}", e)))?;
    let finished = game.finished;
    Ok(Json(TurnResponse {
        state: GameView::from_game(game),
        events,
        finished,
    }))
}

async fn bot_turn(
    State(store): State<GameStore>,
    Path(id): Path<Uuid>,
) -> Result<Json<TurnResponse>, ApiError> {
    let mut games = store.lock().await;
    let game = games.get_mut(&id).ok_or(ApiError::GameNotFound)?;
    if game.finished {
        return Err(ApiError::GameFinished);
    }
    game.ensure_drawable();
    let spec = game.bag.last().cloned().ok_or(ApiError::GameFinished)?;
    let pid = game.current_player as PlayerId;
    let has_meeple = game.players[game.current_player].meeples_remaining > 0;
    let mv = choose_move(&game.board, &spec, pid, has_meeple)
        .ok_or(ApiError::NoLegalGreedyMove)?;
    let events = game
        .play_move(mv)
        .map_err(|e| ApiError::BadMove(format!("{:?}", e)))?;
    let finished = game.finished;
    Ok(Json(TurnResponse {
        state: GameView::from_game(game),
        events,
        finished,
    }))
}

async fn legal_moves(
    State(store): State<GameStore>,
    Path(id): Path<Uuid>,
) -> Result<Json<Vec<GreedyMove>>, ApiError> {
    let mut games = store.lock().await;
    let game = games.get_mut(&id).ok_or(ApiError::GameNotFound)?;
    game.ensure_drawable();
    Ok(Json(game.legal_moves()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::{to_bytes, Body};
    use axum::http::{Method, Request, StatusCode};
    use tower::ServiceExt;

    fn build_app() -> Router {
        router(crate::api::new_store())
    }

    async fn body_to_json(body: Body) -> serde_json::Value {
        let bytes = to_bytes(body, usize::MAX).await.unwrap();
        serde_json::from_slice(&bytes).unwrap()
    }

    #[tokio::test]
    async fn create_then_get_game() {
        let app = build_app();
        let create_resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/games")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"num_players":2,"seed":42}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(create_resp.status(), StatusCode::OK);
        let create_json = body_to_json(create_resp.into_body()).await;
        let game_id = create_json["game_id"].as_str().unwrap().to_string();

        let get_resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(format!("/games/{game_id}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(get_resp.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn bot_turn_advances_state() {
        let app = build_app();
        let create_resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/games")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"num_players":2,"seed":7}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        let create_json = body_to_json(create_resp.into_body()).await;
        let game_id = create_json["game_id"].as_str().unwrap().to_string();
        let bag_before = create_json["state"]["bag_remaining"].as_u64().unwrap();

        let turn_resp = app
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri(format!("/games/{game_id}/bot-turn"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(turn_resp.status(), StatusCode::OK);
        let turn_json = body_to_json(turn_resp.into_body()).await;
        let bag_after = turn_json["state"]["bag_remaining"].as_u64().unwrap();
        assert!(bag_after < bag_before);
    }

    #[tokio::test]
    async fn legal_moves_returns_non_empty_for_fresh_game() {
        let app = build_app();
        let create_resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/games")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"num_players":2,"seed":1}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        let create_json = body_to_json(create_resp.into_body()).await;
        let game_id = create_json["game_id"].as_str().unwrap().to_string();

        let legal_resp = app
            .oneshot(
                Request::builder()
                    .uri(format!("/games/{game_id}/legal-moves"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(legal_resp.status(), StatusCode::OK);
        let legal_json = body_to_json(legal_resp.into_body()).await;
        assert!(legal_json.as_array().unwrap().len() > 0);
    }

    #[tokio::test]
    async fn unknown_game_returns_404() {
        let app = build_app();
        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/games/00000000-0000-0000-0000-000000000000")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }
}
