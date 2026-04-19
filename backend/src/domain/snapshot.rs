use serde::Serialize;

use crate::domain::board::{ActiveMeeple, Board, Pos};
use crate::domain::game::Game;
use crate::domain::player::Player;
use crate::domain::tile::{PlacedTile, TileSpec};

#[derive(Debug, Clone, Serialize)]
pub struct CellView {
    pub pos: Pos,
    pub tile: PlacedTile,
}

#[derive(Debug, Clone, Serialize)]
pub struct BoardView {
    pub cells: Vec<CellView>,
    pub meeples: Vec<ActiveMeeple>,
    pub last_placed: Option<Pos>,
}

#[derive(Debug, Clone, Serialize)]
pub struct GameView {
    pub board: BoardView,
    pub players: Vec<Player>,
    pub bag_remaining: usize,
    pub current_player: u8,
    pub current_draw: Option<TileSpec>,
    pub is_over: bool,
    pub finished: bool,
}

impl BoardView {
    pub fn from_board(board: &Board) -> Self {
        let mut cells: Vec<CellView> = board
            .positions()
            .map(|p| CellView {
                pos: p,
                tile: board.get(p).expect("position from positions() must exist").clone(),
            })
            .collect();
        cells.sort_by_key(|c| (c.pos.0, c.pos.1));
        Self {
            cells,
            meeples: board.meeples().to_vec(),
            last_placed: board.last_placed(),
        }
    }
}

impl GameView {
    pub fn from_game(game: &Game) -> Self {
        Self {
            board: BoardView::from_board(&game.board),
            players: game.players.clone(),
            bag_remaining: game.bag.len(),
            current_player: game.current_player as u8,
            // The next tile drawn will be the LAST in the bag (Vec::pop semantics).
            current_draw: game.bag.last().cloned(),
            is_over: game.is_over(),
            finished: game.finished,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::game::{greedy_bot, BotFn, Game};

    #[test]
    fn empty_game_snapshot_has_one_starter_cell_and_no_meeples() {
        let game = Game::new(2, 1);
        let view = GameView::from_game(&game);
        assert_eq!(view.board.cells.len(), 1);
        assert!(view.board.meeples.is_empty());
        assert_eq!(view.players.len(), 2);
        assert_eq!(view.bag_remaining, 71);
        assert_eq!(view.current_player, 0);
        assert!(view.current_draw.is_some());
        assert!(!view.is_over);
    }

    #[test]
    fn snapshot_serializes_to_json_without_panic() {
        let game = Game::new(2, 7);
        let view = GameView::from_game(&game);
        let json = serde_json::to_string(&view).expect("serialize");
        assert!(json.starts_with('{'));
        assert!(json.contains("\"board\""));
        assert!(json.contains("\"players\""));
        assert!(json.contains("\"bag_remaining\""));
    }

    #[test]
    fn snapshot_after_a_few_turns_has_more_cells() {
        let mut game = Game::new(2, 11);
        let mut bots: Vec<BotFn> = vec![greedy_bot(), greedy_bot()];
        for _ in 0..6 {
            let cp = game.current_player;
            let _ = game.play_one_turn(&mut bots[cp]);
        }
        let view = GameView::from_game(&game);
        assert!(view.board.cells.len() >= 3, "expected several tiles placed");
    }

    #[test]
    fn meeples_are_tracked_after_greedy_play() {
        // Play a full game; meeples returned via scoring should not appear,
        // but late-game meeples still on the board should.
        let mut game = Game::new(2, 13);
        let mut bots: Vec<BotFn> = vec![greedy_bot(), greedy_bot()];
        game.play_full_game(&mut bots);
        // After endgame, all meeples are returned and pruned.
        let view = GameView::from_game(&game);
        assert!(view.is_over);
        assert!(
            view.board.meeples.is_empty(),
            "endgame returns all meeples: {:?}",
            view.board.meeples
        );
    }
}
