use std::collections::HashMap;

use crate::domain::feature::FeatureGraph;
use crate::domain::tile::{edges_match, PlacedTile, Side};

pub type Pos = (i32, i32);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlacementError {
    Occupied,
    NotAdjacent,
    EdgeMismatch(Side),
}

#[derive(Debug, Default)]
pub struct Board {
    tiles: HashMap<Pos, PlacedTile>,
    pub features: FeatureGraph,
}

impl Board {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn is_empty(&self) -> bool {
        self.tiles.is_empty()
    }

    pub fn len(&self) -> usize {
        self.tiles.len()
    }

    pub fn get(&self, pos: Pos) -> Option<&PlacedTile> {
        self.tiles.get(&pos)
    }

    pub fn neighbor(&self, pos: Pos, side: Side) -> Option<&PlacedTile> {
        self.tiles.get(&offset(pos, side))
    }

    pub fn can_place(&self, pos: Pos, tile: &PlacedTile) -> Result<(), PlacementError> {
        if self.tiles.contains_key(&pos) {
            return Err(PlacementError::Occupied);
        }
        if self.tiles.is_empty() {
            return Ok(());
        }
        let mut has_neighbor = false;
        for side in Side::all() {
            if let Some(n) = self.neighbor(pos, side) {
                has_neighbor = true;
                if !edges_match(tile, side, n) {
                    return Err(PlacementError::EdgeMismatch(side));
                }
            }
        }
        if has_neighbor {
            Ok(())
        } else {
            Err(PlacementError::NotAdjacent)
        }
    }

    pub fn place(&mut self, pos: Pos, tile: PlacedTile) -> Result<(), PlacementError> {
        self.can_place(pos, &tile)?;
        let neighbor_segs: Vec<(Side, (Pos, u8))> = Side::all()
            .into_iter()
            .filter_map(|s| {
                let npos = offset(pos, s);
                self.tiles
                    .get(&npos)
                    .map(|nt| (s, (npos, nt.segment_id(s.opposite()))))
            })
            .collect();
        self.features.add_tile(pos, &tile, |side| {
            neighbor_segs
                .iter()
                .find(|(s, _)| *s == side)
                .map(|(_, seg)| *seg)
        });
        self.tiles.insert(pos, tile);
        Ok(())
    }
}

pub fn offset(pos: Pos, side: Side) -> Pos {
    let (x, y) = pos;
    match side {
        Side::North => (x, y + 1),
        Side::East => (x + 1, y),
        Side::South => (x, y - 1),
        Side::West => (x - 1, y),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::tile::{EdgeKind, TileSpec};

    fn straight_road() -> TileSpec {
        TileSpec {
            edges: [EdgeKind::Road, EdgeKind::Field, EdgeKind::Road, EdgeKind::Field],
            segments: [0, 1, 0, 2],
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

    fn city_on_north() -> TileSpec {
        TileSpec {
            edges: [EdgeKind::City, EdgeKind::Field, EdgeKind::Field, EdgeKind::Field],
            segments: [0, 1, 1, 1],
            monastery: false,
            shield: false,
        }
    }

    #[test]
    fn empty_board_accepts_first_tile_anywhere() {
        let mut board = Board::new();
        assert!(board.place((0, 0), PlacedTile::new(straight_road(), 0)).is_ok());
        assert_eq!(board.len(), 1);
    }

    #[test]
    fn cannot_place_on_occupied_position() {
        let mut board = Board::new();
        board.place((0, 0), PlacedTile::new(straight_road(), 0)).unwrap();
        let err = board.place((0, 0), PlacedTile::new(straight_road(), 0)).unwrap_err();
        assert_eq!(err, PlacementError::Occupied);
    }

    #[test]
    fn second_tile_must_be_adjacent_to_existing() {
        let mut board = Board::new();
        board.place((0, 0), PlacedTile::new(straight_road(), 0)).unwrap();
        let err = board.place((5, 5), PlacedTile::new(straight_road(), 0)).unwrap_err();
        assert_eq!(err, PlacementError::NotAdjacent);
    }

    #[test]
    fn second_tile_legal_when_edge_matches() {
        let mut board = Board::new();
        board.place((0, 0), PlacedTile::new(straight_road(), 0)).unwrap();
        // straight_road at (0,0): N=Road, S=Road. Place another at (0,1) (north of origin).
        // The new tile's South must be Road. straight_road rotation 0: S=Road. OK.
        assert!(board.place((0, 1), PlacedTile::new(straight_road(), 0)).is_ok());
    }

    #[test]
    fn second_tile_rejected_when_edge_mismatches() {
        let mut board = Board::new();
        board.place((0, 0), PlacedTile::new(straight_road(), 0)).unwrap();
        // Place city_on_north (rotation 0: S=Field) at (0,1). Need S=Road. Mismatch.
        let err = board
            .place((0, 1), PlacedTile::new(city_on_north(), 0))
            .unwrap_err();
        assert_eq!(err, PlacementError::EdgeMismatch(Side::South));
    }

    #[test]
    fn placement_validates_all_neighboring_sides() {
        // Build T-shape: tiles at (0,0), (0,1), (1,0). Then place at (1,1):
        // it borders (0,1) on West and (1,0) on South; both must match.
        let mut board = Board::new();
        board.place((0, 0), PlacedTile::new(all_field(), 0)).unwrap();
        board.place((0, 1), PlacedTile::new(all_field(), 0)).unwrap();
        board.place((1, 0), PlacedTile::new(all_field(), 0)).unwrap();
        assert!(board.place((1, 1), PlacedTile::new(all_field(), 0)).is_ok());
    }

    #[test]
    fn neighbor_returns_tile_at_offset() {
        // straight_road rotated 1: E=Road, W=Road. Two of them placed side by side.
        let mut board = Board::new();
        board.place((0, 0), PlacedTile::new(straight_road(), 1)).unwrap();
        board.place((1, 0), PlacedTile::new(straight_road(), 1)).unwrap();
        let n = board.neighbor((0, 0), Side::East).unwrap();
        assert_eq!(n.edge(Side::West), EdgeKind::Road);
    }

    #[test]
    fn neighbor_returns_none_when_empty() {
        let board = Board::new();
        assert!(board.neighbor((0, 0), Side::North).is_none());
    }

    #[test]
    fn place_updates_feature_graph_and_merges_neighbors() {
        let mut board = Board::new();
        board.place((0, 0), PlacedTile::new(straight_road(), 0)).unwrap();
        board.place((0, 1), PlacedTile::new(straight_road(), 0)).unwrap();
        // Both N-S roads share segment 0 of their respective tiles; should be merged.
        let r0 = board.features.find(((0, 0), 0));
        let r1 = board.features.find(((0, 1), 0));
        assert_eq!(r0, r1);
        let info = board.features.info(((0, 0), 0));
        assert_eq!(info.tiles, 2);
    }
}
