use std::collections::HashMap;

use crate::domain::feature::{FeatureInfo, PlayerId};
use crate::domain::tile::EdgeKind;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FeatureKind {
    Road,
    City,
    Monastery,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScoringEvent {
    pub kind: FeatureKind,
    pub points: u32,
    pub winners: Vec<PlayerId>,
    pub meeples_returned: Vec<PlayerId>,
}

pub fn score_completed_feature(info: &FeatureInfo) -> Option<ScoringEvent> {
    let (points, kind) = match info.kind {
        EdgeKind::Road => (info.tiles, FeatureKind::Road),
        EdgeKind::City => (info.tiles * 2 + info.shields * 2, FeatureKind::City),
        EdgeKind::Field => return None,
    };
    Some(ScoringEvent {
        kind,
        points,
        winners: majority_owners(&info.meeples),
        meeples_returned: info.meeples.clone(),
    })
}

pub fn score_incomplete_feature(info: &FeatureInfo) -> Option<ScoringEvent> {
    if info.meeples.is_empty() {
        return None;
    }
    let (points, kind) = match info.kind {
        EdgeKind::Road => (info.tiles, FeatureKind::Road),
        EdgeKind::City => (info.tiles + info.shields, FeatureKind::City),
        EdgeKind::Field => return None,
    };
    Some(ScoringEvent {
        kind,
        points,
        winners: majority_owners(&info.meeples),
        meeples_returned: info.meeples.clone(),
    })
}

pub fn score_completed_monastery(owner: PlayerId) -> ScoringEvent {
    ScoringEvent {
        kind: FeatureKind::Monastery,
        points: 9,
        winners: vec![owner],
        meeples_returned: vec![owner],
    }
}

pub fn score_incomplete_monastery(owner: PlayerId, neighbor_count: u8) -> ScoringEvent {
    ScoringEvent {
        kind: FeatureKind::Monastery,
        points: 1 + neighbor_count as u32,
        winners: vec![owner],
        meeples_returned: vec![owner],
    }
}

pub fn majority_owners(meeples: &[PlayerId]) -> Vec<PlayerId> {
    if meeples.is_empty() {
        return Vec::new();
    }
    let mut counts: HashMap<PlayerId, u32> = HashMap::new();
    for &p in meeples {
        *counts.entry(p).or_insert(0) += 1;
    }
    let max = *counts.values().max().unwrap();
    let mut winners: Vec<PlayerId> = counts
        .into_iter()
        .filter(|(_, c)| *c == max)
        .map(|(p, _)| p)
        .collect();
    winners.sort();
    winners
}

#[cfg(test)]
mod tests {
    use super::*;

    fn road_info(tiles: u32, meeples: Vec<PlayerId>) -> FeatureInfo {
        FeatureInfo {
            kind: EdgeKind::Road,
            open_count: 0,
            tiles,
            shields: 0,
            meeples,
        }
    }

    fn city_info(tiles: u32, shields: u32, meeples: Vec<PlayerId>) -> FeatureInfo {
        FeatureInfo {
            kind: EdgeKind::City,
            open_count: 0,
            tiles,
            shields,
            meeples,
        }
    }

    #[test]
    fn road_scores_one_point_per_tile() {
        let event = score_completed_feature(&road_info(3, vec![1])).unwrap();
        assert_eq!(event.kind, FeatureKind::Road);
        assert_eq!(event.points, 3);
        assert_eq!(event.winners, vec![1]);
        assert_eq!(event.meeples_returned, vec![1]);
    }

    #[test]
    fn city_scores_two_per_tile_plus_two_per_shield() {
        let event = score_completed_feature(&city_info(4, 2, vec![1])).unwrap();
        assert_eq!(event.points, 4 * 2 + 2 * 2);
    }

    #[test]
    fn tied_meeples_means_all_tied_winners_share_full_points() {
        let event = score_completed_feature(&city_info(3, 0, vec![1, 2])).unwrap();
        assert_eq!(event.points, 6);
        assert_eq!(event.winners, vec![1, 2]);
    }

    #[test]
    fn majority_winner_takes_all() {
        let event = score_completed_feature(&road_info(5, vec![1, 1, 2])).unwrap();
        assert_eq!(event.winners, vec![1]);
    }

    #[test]
    fn closed_feature_with_no_meeples_has_no_winners_but_still_scores() {
        let event = score_completed_feature(&road_info(3, vec![])).unwrap();
        assert_eq!(event.points, 3);
        assert!(event.winners.is_empty());
    }

    #[test]
    fn incomplete_road_scores_one_per_tile_only_if_owned() {
        assert!(score_incomplete_feature(&road_info(3, vec![])).is_none());
        let event = score_incomplete_feature(&road_info(3, vec![1])).unwrap();
        assert_eq!(event.points, 3);
    }

    #[test]
    fn incomplete_city_scores_one_per_tile_plus_one_per_shield() {
        let event = score_incomplete_feature(&city_info(3, 1, vec![1])).unwrap();
        assert_eq!(event.points, 3 + 1);
    }

    #[test]
    fn completed_monastery_scores_nine() {
        let event = score_completed_monastery(2);
        assert_eq!(event.points, 9);
        assert_eq!(event.winners, vec![2]);
    }

    #[test]
    fn incomplete_monastery_scores_one_plus_neighbors() {
        let event = score_incomplete_monastery(0, 5);
        assert_eq!(event.points, 6);
    }

    #[test]
    fn field_features_do_not_score_via_immediate_or_incomplete() {
        let info = FeatureInfo {
            kind: EdgeKind::Field,
            open_count: 0,
            tiles: 5,
            shields: 0,
            meeples: vec![1],
        };
        assert!(score_completed_feature(&info).is_none());
        assert!(score_incomplete_feature(&info).is_none());
    }
}
