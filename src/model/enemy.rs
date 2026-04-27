use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EnemyPatternDef {
    pub incoming_damage_by_turn: Vec<i32>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnemyPattern {
    pub incoming_damage_by_turn: Vec<i32>,
}

impl EnemyPattern {
    pub fn from_def(def: EnemyPatternDef) -> Self {
        Self {
            incoming_damage_by_turn: def.incoming_damage_by_turn,
        }
    }

    pub fn incoming_damage(&self, turn_index_0: usize) -> i32 {
        self.incoming_damage_by_turn
            .get(turn_index_0)
            .copied()
            .unwrap_or(0)
    }
}
