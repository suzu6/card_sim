use crate::model::{EnemyStatus, Vulnerable};

use super::draw::CardInstance;

#[derive(Debug, Clone)]
pub struct CombatState {
    pub draw_pile: Vec<CardInstance>,
    pub discard_pile: Vec<CardInstance>,
    pub enemy_status: EnemyStatus,
}

impl CombatState {
    pub fn new(draw_pile: Vec<CardInstance>) -> Self {
        Self {
            draw_pile,
            discard_pile: vec![],
            enemy_status: EnemyStatus::new(),
        }
    }

    pub fn apply_vulnerable(&mut self, turns: u8) {
        let cur = self.enemy_status.vulnerable.turns;
        self.enemy_status.vulnerable = Vulnerable::new(cur.saturating_add(turns));
    }

    pub fn tick_end_of_turn(&mut self) {
        self.enemy_status.vulnerable.tick_end_of_turn();
    }
}
