use crate::domain::tile::{EdgeKind, TileSpec};

use EdgeKind::{City as C, Field as F, Road as R};

fn spec(edges: [EdgeKind; 4], segments: [u8; 4], monastery: bool, shield: bool) -> TileSpec {
    TileSpec { edges, segments, monastery, shield }
}

// Tile A: monastery + road south.
pub fn monastery_with_road() -> TileSpec {
    spec([F, F, R, F], [0, 0, 1, 0], true, false)
}

// Tile B: monastery alone.
pub fn monastery_alone() -> TileSpec {
    spec([F, F, F, F], [0, 0, 0, 0], true, false)
}

// Tile C: all-city + shield (single closed city).
pub fn all_city_shield() -> TileSpec {
    spec([C, C, C, C], [0, 0, 0, 0], false, true)
}

// Tile D: city N, road E-W (straight road through), field S. STARTER.
pub fn city_n_road_ew() -> TileSpec {
    spec([C, R, F, R], [0, 1, 2, 1], false, false)
}

// Tile E: city N only.
pub fn city_n_only() -> TileSpec {
    spec([C, F, F, F], [0, 1, 1, 1], false, false)
}

// Tile F: through-city E-W with shield, fields N and S separated.
pub fn through_city_shield() -> TileSpec {
    spec([F, C, F, C], [1, 0, 2, 0], false, true)
}

// Tile G: through-city E-W (no shield).
pub fn through_city() -> TileSpec {
    spec([F, C, F, C], [1, 0, 2, 0], false, false)
}

// Tile H: split city E and W (two disconnected city segments), field N-S connected.
pub fn split_city_ew() -> TileSpec {
    spec([F, C, F, C], [0, 1, 0, 2], false, false)
}

// Tile I: city corner SE (city on E and S connected).
pub fn city_corner_se() -> TileSpec {
    spec([F, C, C, F], [0, 1, 1, 0], false, false)
}

// Tile J: city N + road right-turn (E-S).
pub fn city_n_road_right_turn() -> TileSpec {
    spec([C, R, R, F], [0, 1, 1, 2], false, false)
}

// Tile K: city N + road left-turn (S-W).
pub fn city_n_road_left_turn() -> TileSpec {
    spec([C, F, R, R], [0, 1, 2, 2], false, false)
}

// Tile L: city N + T-junction roads (E, S, W as separate road segments).
pub fn city_n_road_t() -> TileSpec {
    spec([C, R, R, R], [0, 1, 2, 3], false, false)
}

// Tile M: city corner NE + shield, fields S+W connected.
pub fn city_corner_ne_shield() -> TileSpec {
    spec([C, C, F, F], [0, 0, 1, 1], false, true)
}

// Tile N: city corner NE (no shield).
pub fn city_corner_ne() -> TileSpec {
    spec([C, C, F, F], [0, 0, 1, 1], false, false)
}

// Tile O: city corner NW + road corner SE, with shield.
pub fn city_nw_road_se_shield() -> TileSpec {
    spec([C, R, R, C], [0, 1, 1, 0], false, true)
}

// Tile P: city corner NW + road corner SE (no shield).
pub fn city_nw_road_se() -> TileSpec {
    spec([C, R, R, C], [0, 1, 1, 0], false, false)
}

// Tile Q: 3-side city (N+E+W), field S, with shield.
pub fn three_side_city_shield() -> TileSpec {
    spec([C, C, F, C], [0, 0, 1, 0], false, true)
}

// Tile R: 3-side city (N+E+W), field S (no shield).
pub fn three_side_city() -> TileSpec {
    spec([C, C, F, C], [0, 0, 1, 0], false, false)
}

// Tile S: 3-side city + road S, with shield.
pub fn three_side_city_road_shield() -> TileSpec {
    spec([C, C, R, C], [0, 0, 1, 0], false, true)
}

// Tile T: 3-side city + road S (no shield).
pub fn three_side_city_road() -> TileSpec {
    spec([C, C, R, C], [0, 0, 1, 0], false, false)
}

// Tile U: straight road E-W, fields N and S.
pub fn road_straight() -> TileSpec {
    spec([F, R, F, R], [0, 1, 2, 1], false, false)
}

// Tile V: road corner S-W, fields connected (NE).
pub fn road_corner() -> TileSpec {
    spec([F, F, R, R], [0, 0, 1, 1], false, false)
}

// Tile W: road T-junction (E, S, W as separate roads), field N.
pub fn road_t_junction() -> TileSpec {
    spec([F, R, R, R], [0, 1, 2, 3], false, false)
}

// Tile X: 4-way crossroads (4 separate road segments).
pub fn road_crossroads() -> TileSpec {
    spec([R, R, R, R], [0, 1, 2, 3], false, false)
}

pub fn starter_tile() -> TileSpec {
    city_n_road_ew()
}

/// Standard Carcassonne base game (Wrede / Hans im Glück distribution): 72 tiles total.
/// Includes the starter tile (one copy of tile D); caller draws starter first.
pub fn base_game_bag() -> Vec<TileSpec> {
    let counts: &[(fn() -> TileSpec, u8)] = &[
        (monastery_with_road, 2),
        (monastery_alone, 4),
        (all_city_shield, 1),
        (city_n_road_ew, 4),
        (city_n_only, 5),
        (through_city_shield, 2),
        (through_city, 1),
        (split_city_ew, 3),
        (city_corner_se, 2),
        (city_n_road_right_turn, 3),
        (city_n_road_left_turn, 3),
        (city_n_road_t, 3),
        (city_corner_ne_shield, 2),
        (city_corner_ne, 3),
        (city_nw_road_se_shield, 2),
        (city_nw_road_se, 3),
        (three_side_city_shield, 1),
        (three_side_city, 3),
        (three_side_city_road_shield, 2),
        (three_side_city_road, 1),
        (road_straight, 8),
        (road_corner, 9),
        (road_t_junction, 4),
        (road_crossroads, 1),
    ];
    let mut bag = Vec::with_capacity(72);
    for (factory, qty) in counts {
        for _ in 0..*qty {
            bag.push(factory());
        }
    }
    bag
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base_game_bag_contains_72_tiles() {
        assert_eq!(base_game_bag().len(), 72);
    }

    #[test]
    fn shield_tiles_have_shield_flag() {
        assert!(all_city_shield().shield);
        assert!(through_city_shield().shield);
        assert!(city_corner_ne_shield().shield);
        assert!(city_nw_road_se_shield().shield);
        assert!(three_side_city_shield().shield);
        assert!(three_side_city_road_shield().shield);
    }

    #[test]
    fn monastery_tiles_have_monastery_flag() {
        assert!(monastery_alone().monastery);
        assert!(monastery_with_road().monastery);
    }

    #[test]
    fn non_monastery_tiles_have_no_monastery() {
        assert!(!city_n_road_ew().monastery);
        assert!(!road_straight().monastery);
        assert!(!all_city_shield().monastery);
    }

    #[test]
    fn shield_count_in_bag_is_six() {
        // 1+2+2+2+1+2 = 10 of the shield TYPES, but multiplied by their quantities
        // and only counting shield-bearing copies.
        let count = base_game_bag().iter().filter(|t| t.shield).count();
        // C:1 + F:2 + M:2 + O:2 + Q:1 + S:2 = 10 shielded tiles
        assert_eq!(count, 10);
    }

    #[test]
    fn monastery_count_in_bag_is_six() {
        let count = base_game_bag().iter().filter(|t| t.monastery).count();
        // A:2 + B:4 = 6
        assert_eq!(count, 6);
    }

    #[test]
    fn all_24_types_callable() {
        // Smoke test: just call each constructor once.
        let _ = (
            monastery_with_road(), monastery_alone(), all_city_shield(),
            city_n_road_ew(), city_n_only(), through_city_shield(),
            through_city(), split_city_ew(), city_corner_se(),
            city_n_road_right_turn(), city_n_road_left_turn(), city_n_road_t(),
            city_corner_ne_shield(), city_corner_ne(), city_nw_road_se_shield(),
            city_nw_road_se(), three_side_city_shield(), three_side_city(),
            three_side_city_road_shield(), three_side_city_road(),
            road_straight(), road_corner(), road_t_junction(), road_crossroads(),
        );
    }
}
