#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Side {
    North = 0,
    East = 1,
    South = 2,
    West = 3,
}

impl Side {
    pub fn opposite(self) -> Self {
        match self {
            Side::North => Side::South,
            Side::East => Side::West,
            Side::South => Side::North,
            Side::West => Side::East,
        }
    }

    pub fn all() -> [Side; 4] {
        [Side::North, Side::East, Side::South, Side::West]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EdgeKind {
    Road,
    City,
    Field,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TileSpec {
    pub edges: [EdgeKind; 4],
    pub segments: [u8; 4],
    pub monastery: bool,
    pub shield: bool,
}

#[derive(Debug, Clone)]
pub struct PlacedTile {
    pub spec: TileSpec,
    pub rotation: u8,
}

impl PlacedTile {
    pub fn new(spec: TileSpec, rotation: u8) -> Self {
        Self { spec, rotation: rotation % 4 }
    }

    pub fn edge(&self, side: Side) -> EdgeKind {
        let canonical = (side as u8 + 4 - self.rotation) % 4;
        self.spec.edges[canonical as usize]
    }

    pub fn segment_id(&self, side: Side) -> u8 {
        let canonical = (side as u8 + 4 - self.rotation) % 4;
        self.spec.segments[canonical as usize]
    }
}

pub fn edges_match(a: &PlacedTile, side_of_a: Side, b: &PlacedTile) -> bool {
    a.edge(side_of_a) == b.edge(side_of_a.opposite())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn straight_road() -> TileSpec {
        TileSpec {
            edges: [EdgeKind::Road, EdgeKind::Field, EdgeKind::Road, EdgeKind::Field],
            segments: [0, 1, 0, 2],
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
    fn placed_tile_no_rotation_returns_canonical_edges() {
        let placed = PlacedTile::new(straight_road(), 0);
        assert_eq!(placed.edge(Side::North), EdgeKind::Road);
        assert_eq!(placed.edge(Side::East), EdgeKind::Field);
        assert_eq!(placed.edge(Side::South), EdgeKind::Road);
        assert_eq!(placed.edge(Side::West), EdgeKind::Field);
    }

    #[test]
    fn rotation_90_cw_shifts_edges() {
        let placed = PlacedTile::new(city_on_north(), 1);
        assert_eq!(placed.edge(Side::North), EdgeKind::Field);
        assert_eq!(placed.edge(Side::East), EdgeKind::City);
        assert_eq!(placed.edge(Side::South), EdgeKind::Field);
        assert_eq!(placed.edge(Side::West), EdgeKind::Field);
    }

    #[test]
    fn rotation_180_flips_edges() {
        let placed = PlacedTile::new(city_on_north(), 2);
        assert_eq!(placed.edge(Side::North), EdgeKind::Field);
        assert_eq!(placed.edge(Side::South), EdgeKind::City);
    }

    #[test]
    fn rotation_270_cw() {
        let placed = PlacedTile::new(city_on_north(), 3);
        assert_eq!(placed.edge(Side::West), EdgeKind::City);
    }

    #[test]
    fn rotation_4_equals_rotation_0() {
        let placed = PlacedTile::new(city_on_north(), 4);
        assert_eq!(placed.edge(Side::North), EdgeKind::City);
    }

    #[test]
    fn opposite_side_is_involution() {
        for s in Side::all() {
            assert_eq!(s.opposite().opposite(), s);
        }
    }

    #[test]
    fn adjacent_tiles_match_when_edge_kinds_align() {
        let left = PlacedTile::new(straight_road(), 1);
        let right = PlacedTile::new(straight_road(), 1);
        assert!(edges_match(&left, Side::East, &right));
    }

    #[test]
    fn adjacent_tiles_mismatch_when_edge_kinds_differ() {
        let left = PlacedTile::new(straight_road(), 0);
        let right = PlacedTile::new(city_on_north(), 3);
        assert_eq!(left.edge(Side::East), EdgeKind::Field);
        assert_eq!(right.edge(Side::West), EdgeKind::City);
        assert!(!edges_match(&left, Side::East, &right));
    }

    #[test]
    fn segment_id_rotates_with_edges() {
        let placed = PlacedTile::new(straight_road(), 1);
        assert_eq!(placed.segment_id(Side::East), placed.segment_id(Side::West));
    }
}
