use crate::domain::feature::PlayerId;

pub const STARTING_MEEPLES: u8 = 7;

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Player {
    pub id: PlayerId,
    pub score: u32,
    pub meeples_remaining: u8,
}

impl Player {
    pub fn new(id: PlayerId) -> Self {
        Self {
            id,
            score: 0,
            meeples_remaining: STARTING_MEEPLES,
        }
    }

    pub fn try_take_meeple(&mut self) -> bool {
        if self.meeples_remaining == 0 {
            false
        } else {
            self.meeples_remaining -= 1;
            true
        }
    }

    pub fn return_meeple(&mut self) {
        self.meeples_remaining += 1;
    }

    pub fn add_score(&mut self, pts: u32) {
        self.score += pts;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_player_has_seven_meeples_and_zero_score() {
        let p = Player::new(0);
        assert_eq!(p.meeples_remaining, 7);
        assert_eq!(p.score, 0);
    }

    #[test]
    fn try_take_meeple_decrements_until_empty() {
        let mut p = Player::new(0);
        for _ in 0..7 {
            assert!(p.try_take_meeple());
        }
        assert!(!p.try_take_meeple());
        assert_eq!(p.meeples_remaining, 0);
    }

    #[test]
    fn return_meeple_increments() {
        let mut p = Player::new(0);
        p.try_take_meeple();
        p.return_meeple();
        assert_eq!(p.meeples_remaining, 7);
    }
}
