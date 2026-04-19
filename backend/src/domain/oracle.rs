use std::collections::HashMap;

use crate::domain::board::{Board, Pos};
use crate::domain::feature::PlayerId;
use crate::domain::greedy::{choose_move_with_score, GreedyMove, MeepleChoice};
use crate::domain::tile::TileSpec;

/// Free-choice oracle: at each turn, sees the entire remaining bag and picks
/// the (tile, placement, meeple) tuple maximizing immediate score for `player`.
/// Returns the bag index of the chosen tile and the move to play with it.
pub fn choose_free_choice(
    board: &Board,
    bag: &[TileSpec],
    player: PlayerId,
    has_meeple: bool,
) -> Option<(usize, GreedyMove)> {
    let mut spec_to_first_idx: HashMap<&TileSpec, usize> = HashMap::new();
    for (i, spec) in bag.iter().enumerate() {
        spec_to_first_idx.entry(spec).or_insert(i);
    }
    let mut best: Option<(usize, GreedyMove, i32, OracleKey)> = None;
    for (spec, &idx) in &spec_to_first_idx {
        let Some((mv, score)) = choose_move_with_score(board, spec, player, has_meeple) else {
            continue;
        };
        let key = (idx, mv.pos, mv.rotation, meeple_rank(&mv.meeple));
        match &best {
            None => best = Some((idx, mv, score, key)),
            Some((_, _, bs, bk)) if score > *bs || (score == *bs && key < *bk) => {
                best = Some((idx, mv, score, key));
            }
            _ => {}
        }
    }
    best.map(|(i, m, _, _)| (i, m))
}

type OracleKey = (usize, Pos, u8, u8);

fn meeple_rank(m: &Option<MeepleChoice>) -> u8 {
    match m {
        None => 0,
        Some(MeepleChoice::Segment(s)) => 1 + (*s as u8),
        Some(MeepleChoice::Monastery) => 5,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::tile::{EdgeKind, PlacedTile, Side};

    fn straight_road() -> TileSpec {
        TileSpec {
            edges: [EdgeKind::Road, EdgeKind::Field, EdgeKind::Road, EdgeKind::Field],
            segments: [0, 1, 0, 2],
            monastery: false,
            shield: false,
        }
    }

    fn road_end() -> TileSpec {
        TileSpec {
            edges: [EdgeKind::Road, EdgeKind::Field, EdgeKind::Field, EdgeKind::Field],
            segments: [0, 1, 1, 1],
            monastery: false,
            shield: false,
        }
    }

    fn all_field() -> TileSpec {
        TileSpec {
            edges: [EdgeKind::Field; 4],
            segments: [0; 4],
            monastery: false,
            shield: false,
        }
    }

    #[test]
    fn returns_none_on_empty_bag() {
        let board = Board::new();
        let bag: Vec<TileSpec> = Vec::new();
        assert!(choose_free_choice(&board, &bag, 0, false).is_none());
    }

    #[test]
    fn picks_tile_that_closes_road_for_immediate_score() {
        // 2-tile road, south end capped, north end open. Player has a meeple
        // already placed on the road. Bag contains a useless field tile and a
        // road_end. Oracle should prefer road_end (closes for 3 pts).
        let mut board = Board::new();
        board.place((0, 0), PlacedTile::new(straight_road(), 0)).unwrap();
        board.place_meeple_on_segment(Side::North, 1).unwrap();
        board.place((0, -1), PlacedTile::new(road_end(), 0)).unwrap();
        let _ = board.resolve_scoring();

        let bag = vec![all_field(), road_end()];
        let (idx, mv) = choose_free_choice(&board, &bag, 1, false).unwrap();
        assert_eq!(idx, 1);
        assert_eq!(mv.pos, (0, 1));
        assert_eq!(mv.rotation, 2);
    }

    #[test]
    fn dedups_duplicate_specs_and_returns_first_occurrence() {
        let mut board = Board::new();
        board.place((0, 0), PlacedTile::new(straight_road(), 0)).unwrap();
        // Bag has two identical straight_roads — should pick idx 0.
        let bag = vec![straight_road(), straight_road()];
        let (idx, _) = choose_free_choice(&board, &bag, 0, false).unwrap();
        assert_eq!(idx, 0);
    }
}
