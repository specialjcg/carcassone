use std::collections::{HashMap, HashSet};

use crate::domain::feature::{FeatureGraph, MeepleError, PlayerId, SegmentRef};
use crate::domain::scoring::{
    score_completed_feature, score_completed_monastery, score_farm, score_incomplete_feature,
    score_incomplete_monastery, ScoringEvent,
};
use crate::domain::tile::{edges_match, EdgeKind, PlacedTile, Side};

pub type Pos = (i32, i32);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlacementError {
    Occupied,
    NotAdjacent,
    EdgeMismatch(Side),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MeeplePlaceError {
    NoTileJustPlaced,
    SegmentNotOnLastTile,
    NoMonasteryHere,
    Feature(MeepleError),
}

#[derive(Debug, Clone)]
pub struct MonasteryRecord {
    pub owner: Option<PlayerId>,
    pub neighbor_count: u8,
}

#[derive(Debug, Default, Clone)]
pub struct Board {
    tiles: HashMap<Pos, PlacedTile>,
    pub features: FeatureGraph,
    monasteries: HashMap<Pos, MonasteryRecord>,
    last_placed: Option<Pos>,
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
        if tile.spec.monastery {
            let initial_neighbors = chebyshev_offsets()
                .iter()
                .filter(|d| self.tiles.contains_key(&(pos.0 + d.0, pos.1 + d.1)))
                .count() as u8;
            self.monasteries.insert(
                pos,
                MonasteryRecord {
                    owner: None,
                    neighbor_count: initial_neighbors,
                },
            );
        }
        for d in chebyshev_offsets() {
            let npos = (pos.0 + d.0, pos.1 + d.1);
            if let Some(rec) = self.monasteries.get_mut(&npos) {
                rec.neighbor_count += 1;
            }
        }
        self.tiles.insert(pos, tile);
        self.last_placed = Some(pos);
        Ok(())
    }

    pub fn place_meeple_on_segment(
        &mut self,
        side: Side,
        owner: PlayerId,
    ) -> Result<(), MeeplePlaceError> {
        let pos = self.last_placed.ok_or(MeeplePlaceError::NoTileJustPlaced)?;
        let tile = self.tiles.get(&pos).expect("last_placed must point to a tile");
        let sid = tile.segment_id(side);
        self.features
            .place_meeple((pos, sid), owner)
            .map_err(MeeplePlaceError::Feature)
    }

    pub fn place_meeple_on_monastery(
        &mut self,
        owner: PlayerId,
    ) -> Result<(), MeeplePlaceError> {
        let pos = self.last_placed.ok_or(MeeplePlaceError::NoTileJustPlaced)?;
        let rec = self
            .monasteries
            .get_mut(&pos)
            .ok_or(MeeplePlaceError::NoMonasteryHere)?;
        if rec.owner.is_some() {
            return Err(MeeplePlaceError::Feature(MeepleError::FeatureOccupied));
        }
        rec.owner = Some(owner);
        Ok(())
    }

    pub fn resolve_scoring(&mut self) -> Vec<ScoringEvent> {
        let mut events = Vec::new();
        let pos = match self.last_placed {
            Some(p) => p,
            None => return events,
        };
        let tile = self.tiles.get(&pos).expect("last placed must exist").clone();

        // Score features touching the just-placed tile (each unique root scored once).
        let mut seen_roots: HashSet<SegmentRef> = HashSet::new();
        for side in Side::all() {
            let sid = tile.segment_id(side);
            let root = self.features.find((pos, sid));
            if !seen_roots.insert(root) {
                continue;
            }
            if !self.features.is_complete(root) {
                continue;
            }
            let info = self.features.info(root);
            if let Some(ev) = score_completed_feature(&info) {
                self.features.collect_meeples(root);
                events.push(ev);
            }
        }

        // Score completed monasteries at pos and the 8 surrounding cells.
        let mut to_check: Vec<Pos> = chebyshev_offsets()
            .iter()
            .map(|d| (pos.0 + d.0, pos.1 + d.1))
            .collect();
        to_check.push(pos);
        for mpos in to_check {
            let owner = match self.monasteries.get(&mpos) {
                Some(rec) if rec.owner.is_some() && rec.neighbor_count == 8 => rec.owner.unwrap(),
                _ => continue,
            };
            events.push(score_completed_monastery(owner));
            // Mark monastery as scored by clearing its owner so it won't re-score.
            self.monasteries.get_mut(&mpos).unwrap().owner = None;
        }

        events
    }

    pub fn last_placed(&self) -> Option<Pos> {
        self.last_placed
    }

    pub fn positions(&self) -> impl Iterator<Item = Pos> + '_ {
        self.tiles.keys().copied()
    }

    pub fn monastery(&self, pos: Pos) -> Option<&MonasteryRecord> {
        self.monasteries.get(&pos)
    }

    pub fn endgame_scoring(&mut self) -> Vec<ScoringEvent> {
        let mut events = Vec::new();

        // 1. Incomplete roads/cities with meeples.
        let roots = self.features.roots();
        for (root, info) in &roots {
            if info.kind == EdgeKind::Field {
                continue;
            }
            if info.open_count == 0 {
                continue;
            }
            if let Some(ev) = score_incomplete_feature(info) {
                self.features.collect_meeples(*root);
                events.push(ev);
            }
        }

        // 2. Incomplete monasteries.
        let monastery_owners: Vec<(Pos, PlayerId, u8)> = self
            .monasteries
            .iter()
            .filter_map(|(pos, rec)| rec.owner.map(|o| (*pos, o, rec.neighbor_count)))
            .collect();
        for (pos, owner, count) in monastery_owners {
            events.push(score_incomplete_monastery(owner, count));
            self.monasteries.get_mut(&pos).unwrap().owner = None;
        }

        // 3. Farms: for each owned field root, count adjacent COMPLETED city roots.
        let mut field_to_cities: HashMap<SegmentRef, HashSet<SegmentRef>> = HashMap::new();
        let positions: Vec<Pos> = self.tiles.keys().copied().collect();
        for pos in positions {
            let tile = self.tiles.get(&pos).unwrap().clone();
            let mut field_segs = HashSet::new();
            let mut city_segs = HashSet::new();
            for side in Side::all() {
                let sid = tile.segment_id(side);
                match tile.edge(side) {
                    EdgeKind::Field => {
                        field_segs.insert(sid);
                    }
                    EdgeKind::City => {
                        city_segs.insert(sid);
                    }
                    EdgeKind::Road => {}
                }
            }
            for f in &field_segs {
                let f_root = self.features.find((pos, *f));
                for c in &city_segs {
                    let c_root = self.features.find((pos, *c));
                    field_to_cities.entry(f_root).or_default().insert(c_root);
                }
            }
        }
        for (f_root, c_roots) in field_to_cities {
            let info = self.features.info(f_root);
            if info.meeples.is_empty() {
                continue;
            }
            let completed = c_roots
                .iter()
                .filter(|c| {
                    let ci = self.features.info(**c);
                    ci.kind == EdgeKind::City && ci.open_count == 0
                })
                .count() as u32;
            if let Some(ev) = score_farm(completed, &info.meeples) {
                self.features.collect_meeples(f_root);
                events.push(ev);
            }
        }

        events
    }
}

fn chebyshev_offsets() -> [(i32, i32); 8] {
    [
        (-1, -1), (0, -1), (1, -1),
        (-1, 0),           (1, 0),
        (-1, 1),  (0, 1),  (1, 1),
    ]
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

    fn road_end() -> TileSpec {
        TileSpec {
            edges: [EdgeKind::Road, EdgeKind::Field, EdgeKind::Field, EdgeKind::Field],
            segments: [0, 1, 1, 1],
            monastery: false,
            shield: false,
        }
    }

    fn pure_monastery() -> TileSpec {
        TileSpec {
            edges: [EdgeKind::Field; 4],
            segments: [0; 4],
            monastery: true,
            shield: false,
        }
    }

    #[test]
    fn closing_a_road_emits_scoring_event_and_returns_meeple() {
        use crate::domain::scoring::FeatureKind;
        let mut board = Board::new();
        board.place((0, 0), PlacedTile::new(straight_road(), 0)).unwrap();
        board.place_meeple_on_segment(Side::North, 1).unwrap();
        // Cap south end with road_end rotation 0 (road on N) at (0, -1).
        board.place((0, -1), PlacedTile::new(road_end(), 0)).unwrap();
        // No closure yet (north of (0,0) still open).
        assert!(board.resolve_scoring().is_empty());
        // Cap north end at (0, 1) with road_end rotation 2 (road on S).
        board.place((0, 1), PlacedTile::new(road_end(), 2)).unwrap();
        let events = board.resolve_scoring();
        assert_eq!(events.len(), 1);
        let ev = &events[0];
        assert_eq!(ev.kind, FeatureKind::Road);
        assert_eq!(ev.points, 3); // 3 tiles
        assert_eq!(ev.winners, vec![1]);
        assert_eq!(ev.meeples_returned, vec![1]);
    }

    #[test]
    fn cannot_place_meeple_on_segment_when_no_tile_placed() {
        let mut board = Board::new();
        let err = board.place_meeple_on_segment(Side::North, 1).unwrap_err();
        assert_eq!(err, MeeplePlaceError::NoTileJustPlaced);
    }

    #[test]
    fn cannot_place_meeple_on_monastery_when_tile_has_none() {
        let mut board = Board::new();
        board.place((0, 0), PlacedTile::new(straight_road(), 0)).unwrap();
        let err = board.place_meeple_on_monastery(1).unwrap_err();
        assert_eq!(err, MeeplePlaceError::NoMonasteryHere);
    }

    #[test]
    fn monastery_neighbor_count_grows_as_tiles_placed_around() {
        let mut board = Board::new();
        board.place((0, 0), PlacedTile::new(pure_monastery(), 0)).unwrap();
        board.place_meeple_on_monastery(1).unwrap();
        assert_eq!(board.monastery((0, 0)).unwrap().neighbor_count, 0);
        // Place a tile to the north (orthogonal neighbor).
        board.place((0, 1), PlacedTile::new(pure_monastery(), 0)).unwrap();
        assert_eq!(board.monastery((0, 0)).unwrap().neighbor_count, 1);
        // Place a tile diagonally NE.
        board.place((1, 1), PlacedTile::new(pure_monastery(), 0)).unwrap();
        assert_eq!(board.monastery((0, 0)).unwrap().neighbor_count, 2);
    }

    #[test]
    fn endgame_scores_incomplete_road_with_meeple() {
        use crate::domain::scoring::FeatureKind;
        let mut board = Board::new();
        board.place((0, 0), PlacedTile::new(straight_road(), 0)).unwrap();
        board.place_meeple_on_segment(Side::North, 1).unwrap();
        let events = board.endgame_scoring();
        let roads: Vec<_> = events.iter().filter(|e| e.kind == FeatureKind::Road).collect();
        assert_eq!(roads.len(), 1);
        assert_eq!(roads[0].points, 1);
        assert_eq!(roads[0].winners, vec![1]);
    }

    #[test]
    fn endgame_skips_incomplete_features_without_meeples() {
        let mut board = Board::new();
        board.place((0, 0), PlacedTile::new(straight_road(), 0)).unwrap();
        let events = board.endgame_scoring();
        assert!(events.is_empty());
    }

    #[test]
    fn endgame_scores_incomplete_monastery() {
        use crate::domain::scoring::FeatureKind;
        let mut board = Board::new();
        board.place((0, 0), PlacedTile::new(pure_monastery(), 0)).unwrap();
        board.place_meeple_on_monastery(2).unwrap();
        board.place((0, 1), PlacedTile::new(all_field(), 0)).unwrap();
        board.place((0, -1), PlacedTile::new(all_field(), 0)).unwrap();
        board.place((1, 0), PlacedTile::new(all_field(), 0)).unwrap();
        let events = board.endgame_scoring();
        let monastery: Vec<_> = events
            .iter()
            .filter(|e| e.kind == FeatureKind::Monastery)
            .collect();
        assert_eq!(monastery.len(), 1);
        assert_eq!(monastery[0].points, 1 + 3);
        assert_eq!(monastery[0].winners, vec![2]);
    }

    #[test]
    fn endgame_farm_scores_three_per_adjacent_completed_city() {
        use crate::domain::scoring::FeatureKind;
        let mut board = Board::new();
        // A: city on N, field on E/S/W. Meeple on field (south side).
        board.place((0, 0), PlacedTile::new(city_on_north(), 0)).unwrap();
        board.place_meeple_on_segment(Side::South, 1).unwrap();
        let _ = board.resolve_scoring();
        // B at (0,1): city_on_north rotation 2 → city on S. Closes the city.
        board.place((0, 1), PlacedTile::new(city_on_north(), 2)).unwrap();
        let immediate = board.resolve_scoring();
        let cities: Vec<_> = immediate
            .iter()
            .filter(|e| e.kind == FeatureKind::City)
            .collect();
        assert_eq!(cities.len(), 1);
        assert_eq!(cities[0].points, 4);
        // Endgame: farm gets 3 pts for the 1 completed adjacent city.
        let endgame = board.endgame_scoring();
        let farms: Vec<_> = endgame.iter().filter(|e| e.kind == FeatureKind::Farm).collect();
        assert_eq!(farms.len(), 1);
        assert_eq!(farms[0].points, 3);
        assert_eq!(farms[0].winners, vec![1]);
    }

    #[test]
    fn endgame_farm_does_not_score_for_incomplete_city() {
        use crate::domain::scoring::FeatureKind;
        let mut board = Board::new();
        board.place((0, 0), PlacedTile::new(city_on_north(), 0)).unwrap();
        board.place_meeple_on_segment(Side::South, 1).unwrap();
        // City on N is open (never capped). Endgame: farm sees no completed adjacent city.
        let endgame = board.endgame_scoring();
        let farms: Vec<_> = endgame.iter().filter(|e| e.kind == FeatureKind::Farm).collect();
        assert!(farms.is_empty());
    }

    #[test]
    fn completed_monastery_scores_nine_points() {
        use crate::domain::scoring::FeatureKind;
        let mut board = Board::new();
        // Surround (0,0) monastery with 8 field tiles.
        board.place((0, 0), PlacedTile::new(pure_monastery(), 0)).unwrap();
        board.place_meeple_on_monastery(2).unwrap();
        let positions = [
            (1, 0), (-1, 0), (0, 1), (0, -1),
            (1, 1), (1, -1), (-1, 1), (-1, -1),
        ];
        for (i, p) in positions.iter().enumerate() {
            board.place(*p, PlacedTile::new(all_field(), 0)).unwrap();
            let events = board.resolve_scoring();
            if i < 7 {
                // Last placement (the 8th neighbor) triggers monastery completion.
                assert!(
                    events.iter().all(|e| e.kind != FeatureKind::Monastery),
                    "monastery should not score until all 8 neighbors placed (i={i})"
                );
            } else {
                let monastery_events: Vec<_> =
                    events.iter().filter(|e| e.kind == FeatureKind::Monastery).collect();
                assert_eq!(monastery_events.len(), 1);
                assert_eq!(monastery_events[0].points, 9);
                assert_eq!(monastery_events[0].winners, vec![2]);
            }
        }
    }
}
