use rand::rngs::StdRng;
use rand::seq::SliceRandom;
use rand::SeedableRng;

use crate::domain::board::Board;
use crate::domain::feature::PlayerId;
use crate::domain::greedy::{GreedyMove, MeepleChoice};
use crate::domain::player::Player;
use crate::domain::scoring::ScoringEvent;
use crate::domain::tile::{PlacedTile, TileSpec};
use crate::domain::tile_set;

pub type BotFn = Box<dyn FnMut(&Board, &TileSpec, PlayerId, bool) -> Option<GreedyMove>>;

pub struct Game {
    pub board: Board,
    pub players: Vec<Player>,
    pub bag: Vec<TileSpec>,
    pub current_player: usize,
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
        }
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

    pub fn play_full_game(&mut self, bots: &mut [BotFn]) {
        assert_eq!(bots.len(), self.players.len());
        while !self.is_over() {
            let cp = self.current_player;
            let _ = self.play_one_turn(&mut bots[cp]);
        }
        let endgame = self.board.endgame_scoring();
        self.apply_scoring(&endgame);
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
