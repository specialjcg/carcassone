use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::Mutex;
use uuid::Uuid;

use crate::domain::game::Game;

pub type GameStore = Arc<Mutex<HashMap<Uuid, Game>>>;

pub fn new_store() -> GameStore {
    Arc::new(Mutex::new(HashMap::new()))
}
