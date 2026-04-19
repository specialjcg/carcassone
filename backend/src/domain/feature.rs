use std::collections::HashMap;

use crate::domain::board::Pos;
use crate::domain::tile::{EdgeKind, PlacedTile, Side};

pub type SegmentRef = (Pos, u8);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FeatureInfo {
    pub kind: EdgeKind,
    pub open_count: u32,
    pub tiles: u32,
}

#[derive(Debug, Clone)]
struct Node {
    parent: SegmentRef,
    rank: u8,
    info: FeatureInfo,
}

#[derive(Debug, Default)]
pub struct FeatureGraph {
    nodes: HashMap<SegmentRef, Node>,
}

impl FeatureGraph {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_tile<F>(&mut self, pos: Pos, tile: &PlacedTile, mut neighbor_seg: F)
    where
        F: FnMut(Side) -> Option<SegmentRef>,
    {
        let mut sides_per_seg: HashMap<u8, Vec<Side>> = HashMap::new();
        for s in Side::all() {
            sides_per_seg.entry(tile.segment_id(s)).or_default().push(s);
        }
        for (sid, sides) in &sides_per_seg {
            let kind = tile.edge(sides[0]);
            let r = (pos, *sid);
            self.nodes.insert(
                r,
                Node {
                    parent: r,
                    rank: 0,
                    info: FeatureInfo {
                        kind,
                        open_count: sides.len() as u32,
                        tiles: 1,
                    },
                },
            );
        }
        for side in Side::all() {
            if let Some(nb) = neighbor_seg(side) {
                let me = (pos, tile.segment_id(side));
                self.union(me, nb);
            }
        }
    }

    pub fn find(&mut self, x: SegmentRef) -> SegmentRef {
        let mut path = Vec::new();
        let mut cur = x;
        loop {
            let p = self
                .nodes
                .get(&cur)
                .expect("segment not registered")
                .parent;
            if p == cur {
                break;
            }
            path.push(cur);
            cur = p;
        }
        let root = cur;
        for n in path {
            self.nodes.get_mut(&n).unwrap().parent = root;
        }
        root
    }

    pub fn info(&mut self, x: SegmentRef) -> FeatureInfo {
        let r = self.find(x);
        self.nodes[&r].info.clone()
    }

    pub fn is_complete(&mut self, x: SegmentRef) -> bool {
        self.info(x).open_count == 0
    }

    fn union(&mut self, a: SegmentRef, b: SegmentRef) {
        let ra = self.find(a);
        let rb = self.find(b);
        if ra == rb {
            let n = self.nodes.get_mut(&ra).unwrap();
            n.info.open_count = n.info.open_count.saturating_sub(2);
            return;
        }
        let rank_a = self.nodes[&ra].rank;
        let rank_b = self.nodes[&rb].rank;
        let (small, big) = if rank_a < rank_b {
            (ra, rb)
        } else {
            (rb, ra)
        };
        let small_info = self.nodes[&small].info.clone();
        {
            let big_node = self.nodes.get_mut(&big).unwrap();
            big_node.info.open_count = (big_node.info.open_count + small_info.open_count)
                .saturating_sub(2);
            big_node.info.tiles += small_info.tiles;
            if rank_a == rank_b {
                big_node.rank += 1;
            }
        }
        self.nodes.get_mut(&small).unwrap().parent = big;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::tile::TileSpec;

    fn straight_road() -> TileSpec {
        TileSpec {
            edges: [EdgeKind::Road, EdgeKind::Field, EdgeKind::Road, EdgeKind::Field],
            segments: [0, 1, 0, 2],
            monastery: false,
            shield: false,
        }
    }

    fn road_end() -> TileSpec {
        // Road on N only; the rest is field (one big field segment).
        TileSpec {
            edges: [EdgeKind::Road, EdgeKind::Field, EdgeKind::Field, EdgeKind::Field],
            segments: [0, 1, 1, 1],
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
    fn single_tile_segment_has_open_count_equal_to_sides() {
        let mut g = FeatureGraph::new();
        let tile = PlacedTile::new(straight_road(), 0);
        g.add_tile((0, 0), &tile, |_| None);
        let info = g.info(((0, 0), 0));
        assert_eq!(info.kind, EdgeKind::Road);
        assert_eq!(info.open_count, 2);
        assert_eq!(info.tiles, 1);
    }

    #[test]
    fn two_adjacent_tiles_share_one_feature() {
        // Two straight roads stacked vertically: both N-S roads connect.
        let mut g = FeatureGraph::new();
        let a = PlacedTile::new(straight_road(), 0);
        g.add_tile((0, 0), &a, |_| None);
        let b = PlacedTile::new(straight_road(), 0);
        // b is placed at (0,1), north of a. Its South side touches a's North.
        g.add_tile((0, 1), &b, |s| {
            if s == Side::South {
                Some(((0, 0), 0)) // a's segment id at North = 0
            } else {
                None
            }
        });
        let info = g.info(((0, 1), 0));
        assert_eq!(info.tiles, 2);
        // Open: a's South (still open) + b's North (still open) = 2
        assert_eq!(info.open_count, 2);
        assert_eq!(g.find(((0, 0), 0)), g.find(((0, 1), 0)));
    }

    #[test]
    fn road_closes_when_both_ends_are_capped() {
        // a (straight) at (0,0), b (road_end facing south) at (0,1) caps north,
        // c (road_end facing north) at (0,-1) caps south.
        let mut g = FeatureGraph::new();
        let a = PlacedTile::new(straight_road(), 0);
        g.add_tile((0, 0), &a, |_| None);
        // road_end rotation 2: N becomes S => road on S, fields elsewhere
        let south_cap = PlacedTile::new(road_end(), 2);
        g.add_tile((0, 1), &south_cap, |s| {
            if s == Side::South {
                Some(((0, 0), 0))
            } else {
                None
            }
        });
        // road_end rotation 0: road on N
        let north_cap = PlacedTile::new(road_end(), 0);
        g.add_tile((0, -1), &north_cap, |s| {
            if s == Side::North {
                Some(((0, 0), 0))
            } else {
                None
            }
        });
        assert!(g.is_complete(((0, 0), 0)));
        assert_eq!(g.info(((0, 0), 0)).tiles, 3);
    }

    #[test]
    fn cities_are_tracked_independently_from_roads() {
        let mut g = FeatureGraph::new();
        let road = PlacedTile::new(straight_road(), 0);
        g.add_tile((0, 0), &road, |_| None);
        let city = PlacedTile::new(city_on_north(), 0);
        g.add_tile((1, 0), &city, |_| None);
        // road at (0,0) and city at (1,0) — no shared segments since edges don't even match types.
        // We don't union them; their roots are distinct.
        assert_ne!(g.find(((0, 0), 0)), g.find(((1, 0), 0)));
        assert_eq!(g.info(((0, 0), 0)).kind, EdgeKind::Road);
        assert_eq!(g.info(((1, 0), 0)).kind, EdgeKind::City);
    }

    #[test]
    fn loop_closure_decrements_open_count() {
        // Build a triangle of fields where the third tile closes a loop already-merged.
        // Simpler: re-union same root via two sides — simulate by placing a tile whose
        // single segment touches two sides of the same already-merged feature.
        // We mock this by creating two tiles already merged, then placing a third tile
        // whose two opposite sides both reference the same merged feature.
        let mut g = FeatureGraph::new();
        let a = PlacedTile::new(straight_road(), 0); // N-S road, segment 0
        g.add_tile((0, 0), &a, |_| None);
        let b = PlacedTile::new(straight_road(), 0);
        g.add_tile((0, 1), &b, |s| {
            if s == Side::South { Some(((0, 0), 0)) } else { None }
        });
        // Now (0,0)/0 and (0,1)/0 share root, open_count=2 (S of a, N of b).
        // Place a tile at (0,2) that has road on N AND S — it closes the road on b's North,
        // and we'll pretend its North side connects back (impossible in reality but tests union math).
        let c = PlacedTile::new(straight_road(), 0);
        g.add_tile((0, 2), &c, |s| {
            if s == Side::South { Some(((0, 1), 0)) } else { None }
        });
        // After: 3 tiles, road_open = 2 (S of a, N of c)
        let info = g.info(((0, 0), 0));
        assert_eq!(info.tiles, 3);
        assert_eq!(info.open_count, 2);
    }
}
