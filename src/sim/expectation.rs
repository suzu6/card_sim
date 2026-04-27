use crate::model::{CardCatalog, CardKind, Deck, EnemyPattern};

use super::{
    combat_state::CombatState,
    draw::{build_draw_pile, draw_hand_random_5, validate_deck, CardInstance},
    evaluate::compute_turn_result,
    play::end_turn_discard_all_hand,
};
use rand::{rngs::StdRng, SeedableRng};

#[derive(Debug, Clone)]
pub struct SimulationInput<'a> {
    pub catalog: &'a CardCatalog,
    pub deck: &'a Deck,
    pub enemy: &'a EnemyPattern,
    pub turns: usize,
    pub samples: u32,
    pub seed: u64,
}

#[derive(Debug, Clone, Copy)]
pub struct SimulationOutput {
    pub turn1_expected_damage: f64,
    pub turn1_expected_block: f64,
    pub total_expected_damage: f64,
    pub total_expected_block: f64,
    pub total_expected_taken: f64,
}

fn kind(catalog: &CardCatalog, inst: &CardInstance) -> CardKind {
    catalog
        .get(&inst.card_id)
        .unwrap_or_else(|| panic!("未知のcard_id: {}", inst.card_id))
        .kind
}

fn cost(catalog: &CardCatalog, inst: &CardInstance) -> u8 {
    catalog
        .get(&inst.card_id)
        .unwrap_or_else(|| panic!("未知のcard_id: {}", inst.card_id))
        .cost
}

fn greedy_play_ids_in_order(
    catalog: &CardCatalog,
    state: &CombatState,
    hand: &[CardInstance],
    energy: u8,
) -> Vec<String> {
    // 方針:
    // 1) Bashがあり、かつBash後に攻撃を追加で打てるなら先にBash
    // 2) 残りエナジーで攻撃を可能な限りプレイ（順序は任意だが、脆弱を先に付与できるBashは特別扱い済み）
    // 3) 余ったエナジーでブロック
    let mut remaining = energy;
    let mut played = vec![];
    let mut used = vec![false; hand.len()];

    let bash_idx = hand.iter().position(|c| c.card_id == "bash");
    if let Some(i) = bash_idx {
        let bash_cost = cost(catalog, &hand[i]);
        if remaining >= bash_cost {
            let can_follow_attack = hand.iter().enumerate().any(|(j, c)| {
                if j == i {
                    return false;
                }
                kind(catalog, c) == CardKind::Attack && cost(catalog, c) <= remaining - bash_cost
            });
            if can_follow_attack {
                used[i] = true;
                remaining -= bash_cost;
                played.push(hand[i].card_id.clone());
            }
        }
    }

    // 攻撃
    loop {
        let mut picked: Option<usize> = None;
        for (i, c) in hand.iter().enumerate() {
            if used[i] {
                continue;
            }
            if kind(catalog, c) != CardKind::Attack {
                continue;
            }
            let c_cost = cost(catalog, c);
            if c_cost <= remaining {
                picked = Some(i);
                break;
            }
        }
        if let Some(i) = picked {
            used[i] = true;
            remaining -= cost(catalog, &hand[i]);
            played.push(hand[i].card_id.clone());
        } else {
            break;
        }
    }

    // ブロック
    loop {
        let mut picked: Option<usize> = None;
        for (i, c) in hand.iter().enumerate() {
            if used[i] {
                continue;
            }
            if kind(catalog, c) != CardKind::Skill {
                continue;
            }
            let c_cost = cost(catalog, c);
            if c_cost <= remaining {
                picked = Some(i);
                break;
            }
        }
        if let Some(i) = picked {
            used[i] = true;
            remaining -= cost(catalog, &hand[i]);
            played.push(hand[i].card_id.clone());
        } else {
            break;
        }
    }

    // Curseはプレイ不可なので無視
    let _ = state;
    played
}

fn run_one_combat(
    catalog: &CardCatalog,
    enemy: &EnemyPattern,
    initial_draw_pile: &[CardInstance],
    turns: usize,
    rng: &mut StdRng,
) -> (f64, f64, f64, f64, f64) {
    let mut state = CombatState::new(initial_draw_pile.to_vec());
    let mut total_damage = 0.0;
    let mut total_block = 0.0;
    let mut total_taken = 0.0;
    let mut turn1_damage = 0.0;
    let mut turn1_block = 0.0;

    for t in 0..turns {
        let incoming = enemy.incoming_damage(t);
        let hand = draw_hand_random_5(&mut state.draw_pile, &mut state.discard_pile, rng);
        let played_ids = greedy_play_ids_in_order(catalog, &state, &hand, 3);
        let (after_play, turn_result) = compute_turn_result(catalog, &state, &played_ids, incoming);
        state = after_play;

        if t == 0 {
            turn1_damage = turn_result.damage;
            turn1_block = turn_result.block;
        }

        total_damage += turn_result.damage;
        total_block += turn_result.block;
        total_taken += turn_result.taken;

        end_turn_discard_all_hand(&mut state, &hand);
        state.tick_end_of_turn();
    }

    (turn1_damage, turn1_block, total_damage, total_block, total_taken)
}

pub fn simulate(input: SimulationInput<'_>) -> Result<SimulationOutput, String> {
    validate_deck(input.deck, input.catalog)?;

    let draw_pile = build_draw_pile(input.deck);
    if draw_pile.is_empty() {
        return Err("デッキが空です".to_string());
    }

    if input.samples == 0 {
        return Err("samplesは1以上にしてください".to_string());
    }

    let mut rng = StdRng::seed_from_u64(input.seed);

    let mut acc_turn1_damage = 0.0;
    let mut acc_turn1_block = 0.0;
    let mut acc_total_damage = 0.0;
    let mut acc_total_block = 0.0;
    let mut acc_total_taken = 0.0;

    for _ in 0..input.samples {
        let (t1d, t1b, td, tb, tt) =
            run_one_combat(input.catalog, input.enemy, &draw_pile, input.turns, &mut rng);
        acc_turn1_damage += t1d;
        acc_turn1_block += t1b;
        acc_total_damage += td;
        acc_total_block += tb;
        acc_total_taken += tt;
    }

    let n = input.samples as f64;
    Ok(SimulationOutput {
        turn1_expected_damage: acc_turn1_damage / n,
        turn1_expected_block: acc_turn1_block / n,
        total_expected_damage: acc_total_damage / n,
        total_expected_block: acc_total_block / n,
        total_expected_taken: acc_total_taken / n,
    })
}
