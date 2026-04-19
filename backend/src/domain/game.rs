use rand::rngs::StdRng;
use rand::seq::SliceRandom;
use rand::SeedableRng;

use crate::domain::board::{Board, PlacementError};
use crate::domain::feature::PlayerId;
use crate::domain::greedy::{GreedyMove, MeepleChoice};
use crate::domain::player::Player;
use crate::domain::scoring::ScoringEvent;
use crate::domain::tile::{PlacedTile, TileSpec};
use crate::domain::tile_set;

pub type BotFn = Box<dyn FnMut(&Board, &TileSpec, PlayerId, bool) -> Option<GreedyMove>>;
pub type OracleFn =
    Box<dyn FnMut(&Board, &[TileSpec], PlayerId, bool) -> Option<(usize, GreedyMove)>>;

pub struct Game {
    pub board: Board,
    pub players: Vec<Player>,
    pub bag: Vec<TileSpec>,
    pub current_player: usize,
    pub finished: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlayMoveError {
    BagEmpty,
    GameFinished,
    Placement(PlacementError),
}

impl Game {
    pub fn new(num_players: u8, seed: u64) -> Self {
        assert!(num_players > 0);
        let mut rng = StdRng::seed_from_u64(seed);
        let mut bag = tile_set::base_game_bag();
        bag.shuffle(&mut rng);
        let starter = tile_set::starter_tile();
        if let Some(idx) = bag.iter().position(|t| *t == starter) {
            bag.remove(idx);
        }
        let mut board = Board::new();
        board.place((0, 0), PlacedTile::new(starter, 0)).unwrap();
        Self {
            board,
            players: (0..num_players).map(Player::new).collect(),
            bag,
            current_player: 0,
            finished: false,
        }
    }

    /// Drop tiles from the top of the bag until one is playable. Returns a reference
    /// to that tile, or None if the bag is exhausted.
    pub fn ensure_drawable(&mut self) -> Option<&TileSpec> {
        while let Some(top) = self.bag.last().cloned() {
            if self.is_playable(&top) {
                return self.bag.last();
            }
            self.bag.pop();
        }
        None
    }

    fn is_playable(&self, spec: &TileSpec) -> bool {
        let candidates: Vec<_> = if self.board.is_empty() {
            vec![(0, 0)]
        } else {
            use crate::domain::board::offset;
            use crate::domain::tile::Side;
            use std::collections::HashSet;
            let occ: HashSet<_> = self.board.positions().collect();
            let mut out: HashSet<_> = HashSet::new();
            for p in &occ {
                for s in Side::all() {
                    let np = offset(*p, s);
                    if !occ.contains(&np) {
                        out.insert(np);
                    }
                }
            }
            out.into_iter().collect()
        };
        candidates.iter().any(|&pos| {
            (0..4u8).any(|rot| self.board.can_place(pos, &PlacedTile::new(spec.clone(), rot)).is_ok())
        })
    }

    /// Apply an explicit move using the current top of the bag (the "drawn" tile).
    /// Removes that tile from the bag, resolves scoring, advances the current player,
    /// and auto-runs endgame when no playable tiles remain.
    pub fn play_move(&mut self, mv: GreedyMove) -> Result<Vec<ScoringEvent>, PlayMoveError> {
        if self.finished {
            return Err(PlayMoveError::GameFinished);
        }
        let spec = self.bag.last().cloned().ok_or(PlayMoveError::BagEmpty)?;
        let placed = PlacedTile::new(spec, mv.rotation);
        self.board.place(mv.pos, placed).map_err(PlayMoveError::Placement)?;
        let pid = self.current_player as PlayerId;
        if let Some(choice) = mv.meeple.clone() {
            let placed_ok = match choice {
                MeepleChoice::Segment(s) => self.board.place_meeple_on_segment(s, pid).is_ok(),
                MeepleChoice::Monastery => self.board.place_meeple_on_monastery(pid).is_ok(),
            };
            if placed_ok {
                self.players[self.current_player].try_take_meeple();
            }
        }
        self.bag.pop();
        let mut events = self.board.resolve_scoring();
        self.apply_scoring(&events);
        self.current_player = (self.current_player + 1) % self.players.len();
        self.ensure_drawable();
        if self.bag.is_empty() && !self.finished {
            let endgame = self.board.endgame_scoring();
            self.apply_scoring(&endgame);
            events.extend(endgame);
            self.finished = true;
        }
        Ok(events)
    }

    /// Enumerate all legal moves for the current top of the bag (for the current player).
    pub fn legal_moves(&self) -> Vec<GreedyMove> {
        let Some(spec) = self.bag.last() else {
            return Vec::new();
        };
        let has_meeple = self.players[self.current_player].meeples_remaining > 0;
        crate::domain::random::enumerate_legal(&self.board, spec, has_meeple)
    }

    pub fn is_over(&self) -> bool {
        self.bag.is_empty()
    }

    /// Play one turn for the current player. If a drawn tile has no legal placement,
    /// it is discarded and the next is drawn. Advances current_player at the end.
    pub fn play_one_turn(&mut self, bot: &mut BotFn) -> Vec<ScoringEvent> {
        let mut events = Vec::new();
        loop {
            let tile_spec = match self.bag.pop() {
                Some(t) => t,
                None => return events,
            };
            let pid = self.current_player as PlayerId;
            let has_meeple = self.players[self.current_player].meeples_remaining > 0;
            let mv = bot(&self.board, &tile_spec, pid, has_meeple);
            match mv {
                Some(m) => {
                    let placed = PlacedTile::new(tile_spec, m.rotation);
                    self.board
                        .place(m.pos, placed)
                        .expect("bot returned an illegal move");
                    if let Some(choice) = m.meeple {
                        let placed_ok = match choice {
                            MeepleChoice::Segment(s) => {
                                self.board.place_meeple_on_segment(s, pid).is_ok()
                            }
                            MeepleChoice::Monastery => {
                                self.board.place_meeple_on_monastery(pid).is_ok()
                            }
                        };
                        if placed_ok {
                            self.players[self.current_player].try_take_meeple();
                        }
                    }
                    let turn_events = self.board.resolve_scoring();
                    self.apply_scoring(&turn_events);
                    events.extend(turn_events);
                    break;
                }
                None => continue,
            }
        }
        self.current_player = (self.current_player + 1) % self.players.len();
        events
    }

    /// Free-choice variant: oracle picks any tile from the bag at each turn.
    /// Skips the per-tile redraw loop because the oracle inspects the whole bag.
    pub fn play_one_oracle_turn(&mut self, oracle: &mut OracleFn) -> Vec<ScoringEvent> {
        let mut events = Vec::new();
        if self.bag.is_empty() {
            return events;
        }
        let pid = self.current_player as PlayerId;
        let has_meeple = self.players[self.current_player].meeples_remaining > 0;
        match oracle(&self.board, &self.bag, pid, has_meeple) {
            Some((idx, m)) => {
                let tile_spec = self.bag.remove(idx);
                let placed = PlacedTile::new(tile_spec, m.rotation);
                self.board
                    .place(m.pos, placed)
                    .expect("oracle returned an illegal move");
                if let Some(choice) = m.meeple {
                    let placed_ok = match choice {
                        MeepleChoice::Segment(s) => {
                            self.board.place_meeple_on_segment(s, pid).is_ok()
                        }
                        MeepleChoice::Monastery => {
                            self.board.place_meeple_on_monastery(pid).is_ok()
                        }
                    };
                    if placed_ok {
                        self.players[self.current_player].try_take_meeple();
                    }
                }
                let turn_events = self.board.resolve_scoring();
                self.apply_scoring(&turn_events);
                events.extend(turn_events);
                self.current_player = (self.current_player + 1) % self.players.len();
            }
            None => {
                // No tile in the bag has any legal placement. Drop one to make progress.
                self.bag.pop();
            }
        }
        events
    }

    pub fn finish(&mut self) {
        if self.finished {
            return;
        }
        let endgame = self.board.endgame_scoring();
        self.apply_scoring(&endgame);
        self.finished = true;
    }

    pub fn play_full_game(&mut self, bots: &mut [BotFn]) {
        assert_eq!(bots.len(), self.players.len());
        while !self.is_over() {
            let cp = self.current_player;
            let _ = self.play_one_turn(&mut bots[cp]);
        }
        self.finish();
    }

    fn apply_scoring(&mut self, events: &[ScoringEvent]) {
        for ev in events {
            for w in &ev.winners {
                self.players[*w as usize].add_score(ev.points);
            }
            for owner in &ev.meeples_returned {
                self.players[*owner as usize].return_meeple();
            }
        }
    }

    pub fn final_scores(&self) -> Vec<u32> {
        self.players.iter().map(|p| p.score).collect()
    }
}

pub fn greedy_bot() -> BotFn {
    Box::new(|board, spec, pid, hm| crate::domain::greedy::choose_move(board, spec, pid, hm))
}

pub fn random_bot(seed: u64) -> BotFn {
    let mut counter = seed;
    Box::new(move |board, spec, pid, hm| {
        let s = counter;
        counter = counter.wrapping_add(1);
        crate::domain::random::choose_move_seeded(board, spec, pid, hm, s)
    })
}

pub fn oracle_bot() -> OracleFn {
    Box::new(|board, bag, pid, hm| crate::domain::oracle::choose_free_choice(board, bag, pid, hm))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_game_places_starter_at_origin_and_drops_one_from_bag() {
        let game = Game::new(2, 42);
        assert_eq!(game.board.len(), 1);
        assert!(game.board.get((0, 0)).is_some());
        // Bag had 72 tiles, one starter removed → 71 remaining.
        assert_eq!(game.bag.len(), 71);
        assert_eq!(game.players.len(), 2);
    }

    #[test]
    fn greedy_vs_greedy_terminates_with_valid_scores() {
        let mut game = Game::new(2, 7);
        let mut bots: Vec<BotFn> = vec![greedy_bot(), greedy_bot()];
        game.play_full_game(&mut bots);
        assert!(game.is_over());
        let scores = game.final_scores();
        assert_eq!(scores.len(), 2);
        assert!(scores.iter().sum::<u32>() > 0);
    }

    #[test]
    fn same_seed_produces_identical_scores() {
        let mut g1 = Game::new(2, 123);
        let mut g2 = Game::new(2, 123);
        let mut bots1: Vec<BotFn> = vec![greedy_bot(), greedy_bot()];
        let mut bots2: Vec<BotFn> = vec![greedy_bot(), greedy_bot()];
        g1.play_full_game(&mut bots1);
        g2.play_full_game(&mut bots2);
        assert_eq!(g1.final_scores(), g2.final_scores());
    }

    #[test]
    fn play_move_advances_player_and_pops_bag() {
        let mut game = Game::new(2, 5);
        let before_bag = game.bag.len();
        let mvs = game.legal_moves();
        let mv = mvs.into_iter().next().expect("at least one legal move");
        let _events = game.play_move(mv).unwrap();
        assert_eq!(game.current_player, 1);
        assert!(game.bag.len() < before_bag);
    }

    #[test]
    fn play_move_rejects_after_finished() {
        let mut game = Game::new(2, 9);
        let mut bots: Vec<BotFn> = vec![greedy_bot(), greedy_bot()];
        game.play_full_game(&mut bots);
        assert!(game.finished);
        let dummy = GreedyMove { pos: (10, 10), rotation: 0, meeple: None };
        assert_eq!(game.play_move(dummy), Err(PlayMoveError::GameFinished));
    }

    #[test]
    fn legal_moves_lists_at_least_one_for_fresh_game() {
        let game = Game::new(2, 3);
        assert!(!game.legal_moves().is_empty());
    }

    #[test]
    fn ensure_drawable_returns_some_for_fresh_game() {
        let mut game = Game::new(2, 4);
        assert!(game.ensure_drawable().is_some());
    }

    #[test]
    fn different_seeds_produce_different_outcomes() {
        let mut g1 = Game::new(2, 1);
        let mut g2 = Game::new(2, 2);
        let mut bots1: Vec<BotFn> = vec![greedy_bot(), greedy_bot()];
        let mut bots2: Vec<BotFn> = vec![greedy_bot(), greedy_bot()];
        g1.play_full_game(&mut bots1);
        g2.play_full_game(&mut bots2);
        assert_ne!(g1.final_scores(), g2.final_scores());
    }
}
