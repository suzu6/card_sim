use crate::model::{CardCatalog, CardKind};

use super::{combat_state::CombatState, draw::CardInstance, evaluate::can_play};

#[derive(Debug, Clone)]
pub struct PlaySequence {
    pub played_in_order: Vec<CardInstance>,
    pub energy_spent: u8,
}

fn card_cost(catalog: &CardCatalog, inst: &CardInstance) -> u8 {
    catalog
        .get(&inst.card_id)
        .unwrap_or_else(|| panic!("未知のcard_id: {}", inst.card_id))
        .cost_for(inst.upgraded)
}

fn card_kind(catalog: &CardCatalog, inst: &CardInstance) -> CardKind {
    catalog
        .get(&inst.card_id)
        .unwrap_or_else(|| panic!("未知のcard_id: {}", inst.card_id))
        .kind
}

pub fn enumerate_play_sequences(
    catalog: &CardCatalog,
    hand: &[CardInstance],
    energy: u8,
) -> Vec<PlaySequence> {
    // 「カードをプレイしない」も含める
    let mut out = vec![PlaySequence {
        played_in_order: vec![],
        energy_spent: 0,
    }];

    fn rec(
        catalog: &CardCatalog,
        hand: &[CardInstance],
        used: &mut Vec<bool>,
        energy_left: u8,
        cur: &mut Vec<CardInstance>,
        out: &mut Vec<PlaySequence>,
        energy_spent: u8,
    ) {
        for i in 0..hand.len() {
            if used[i] {
                continue;
            }
            let kind = card_kind(catalog, &hand[i]);
            if !can_play(kind) {
                continue;
            }
            let cost = card_cost(catalog, &hand[i]);
            if cost > energy_left {
                continue;
            }

            used[i] = true;
            cur.push(hand[i].clone());
            out.push(PlaySequence {
                played_in_order: cur.clone(),
                energy_spent: energy_spent + cost,
            });
            rec(
                catalog,
                hand,
                used,
                energy_left - cost,
                cur,
                out,
                energy_spent + cost,
            );
            cur.pop();
            used[i] = false;
        }
    }

    let mut used = vec![false; hand.len()];
    let mut cur = vec![];
    rec(catalog, hand, &mut used, energy, &mut cur, &mut out, 0);
    out
}

pub fn end_turn_discard_all_hand(state: &mut CombatState, hand: &[CardInstance]) {
    state.discard_pile.extend(hand.iter().cloned());
}
