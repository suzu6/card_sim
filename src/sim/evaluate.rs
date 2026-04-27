use crate::model::{CardCatalog, CardKind};

use super::combat_state::CombatState;

pub fn apply_vulnerable_damage_multiplier(base: i32) -> i32 {
    // Slay the Spire系: 脆弱は与ダメ1.5倍（端数切り捨て）
    (base * 3) / 2
}

#[derive(Debug, Clone, Copy)]
pub struct TurnResult {
    pub damage: f64,
    pub block: f64,
    pub taken: f64,
}

#[derive(Debug, Clone, Copy)]
pub struct ScoreWeights {
    pub damage: f64,
    pub taken: f64,
}

impl ScoreWeights {
    pub fn default_survival_first() -> Self {
        Self {
            damage: 1.0,
            taken: 1000.0,
        }
    }
}

#[derive(Debug, Clone)]
pub struct PlayContext<'a> {
    pub catalog: &'a CardCatalog,
    pub incoming_damage: i32,
    pub weights: ScoreWeights,
}

pub fn can_play(kind: CardKind) -> bool {
    match kind {
        CardKind::Attack | CardKind::Skill => true,
        CardKind::Curse => false,
    }
}

pub fn score_turn(result: &TurnResult, weights: ScoreWeights) -> f64 {
    weights.damage * result.damage - weights.taken * result.taken
}

pub fn compute_turn_result(
    catalog: &CardCatalog,
    state: &CombatState,
    played_card_ids_in_order: &[String],
    incoming_damage: i32,
) -> (CombatState, TurnResult) {
    let mut next_state = state.clone();
    let mut block: i32 = 0;
    let mut damage: i32 = 0;

    for card_id in played_card_ids_in_order {
        let card = catalog
            .get(card_id)
            .unwrap_or_else(|| panic!("未知のcard_id: {}", card_id));

        match card.kind {
            CardKind::Curse => {
                continue;
            }
            CardKind::Skill => {
                block += card.base_block;
            }
            CardKind::Attack => {
                let mut d = card.base_damage;
                if next_state.enemy_status.vulnerable.is_active() {
                    d = apply_vulnerable_damage_multiplier(d);
                }
                damage += d;
                if card.apply_vulnerable > 0 {
                    next_state.apply_vulnerable(card.apply_vulnerable);
                }
            }
        }
    }

    let taken = (incoming_damage - block).max(0);

    (next_state, TurnResult {
        damage: damage as f64,
        block: block as f64,
        taken: taken as f64,
    })
}
