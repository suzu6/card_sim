use crate::model::{CardCatalog, CardEffect, CardKind, Deck, EnemyPattern};

use super::{
    combat_state::CombatState,
    draw::{build_draw_pile, validate_deck, CardInstance},
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
        .cost_for(inst.upgraded)
}

fn greedy_play_ids_in_order(
    catalog: &CardCatalog,
    state: &CombatState,
    hand: &[CardInstance],
    energy: u8,
) -> Vec<CardInstance> {
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
                played.push(hand[i].clone());
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
            played.push(hand[i].clone());
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
            played.push(hand[i].clone());
        } else {
            break;
        }
    }

    // Curseはプレイ不可なので無視
    let _ = state;
    played
}

fn draw_one(state: &mut CombatState, rng: &mut StdRng) -> Option<CardInstance> {
    use rand::seq::SliceRandom;
    if state.draw_pile.is_empty() {
        if state.discard_pile.is_empty() {
            return None;
        }
        state.discard_pile.shuffle(rng);
        state.draw_pile.append(&mut state.discard_pile);
    }
    state.draw_pile.pop()
}

fn draw_n(state: &mut CombatState, n: u8, rng: &mut StdRng) -> Vec<CardInstance> {
    let mut out = vec![];
    for _ in 0..n {
        if let Some(c) = draw_one(state, rng) {
            out.push(c);
        } else {
            break;
        }
    }
    out
}

fn simulate_one_turn_greedy(
    catalog: &CardCatalog,
    state: &CombatState,
    incoming_damage: i32,
    rng: &mut StdRng,
) -> (CombatState, f64, f64, f64) {
    let mut next_state = state.clone();

    // 初期手札5枚
    let mut hand = draw_n(&mut next_state, 5, rng);

    let mut energy: i32 = 3;
    let mut block: i32 = 0;
    let mut damage: i32 = 0;
    let mut self_hp_loss: i32 = 0;
    let mut strength_this_turn: i32 = 0;

    // 逐次プレイ: 既存の「Bash優先→攻撃→ブロック」の順序を基本にしつつ、
    // ドロー等で手札が増えた場合も処理できるよう、1枚ずつ確定して適用する。
    let mut played: Vec<CardInstance> = vec![];

    // まずは既存ロジックを使って初期手札から候補順序を作り、順に実行。
    // 実行中に手札が増えることがあるため、尽きたら再評価する。
    loop {
        if energy <= 0 {
            break;
        }
        let plan = greedy_play_ids_in_order(catalog, &next_state, &hand, energy as u8);
        if plan.is_empty() {
            break;
        }

        // planの先頭から1枚ずつ実行（手札から除去）
        let inst = plan[0].clone();
        let idx = hand.iter().position(|c| c.instance_id == inst.instance_id);
        let Some(hand_idx) = idx else { break };
        let inst = hand.remove(hand_idx);

        let card = catalog
            .get(&inst.card_id)
            .unwrap_or_else(|| panic!("未知のcard_id: {}", inst.card_id));
        let c_cost = card.cost_for(inst.upgraded) as i32;
        if c_cost > energy {
            // 手札にあるがエナジーが足りないケースはスキップ
            continue;
        }
        energy -= c_cost;
        played.push(inst.clone());

        // effectsが空なら従来フィールド互換
        let effects: Vec<CardEffect> = if card.effects_for(inst.upgraded).is_empty() {
            let mut v = vec![];
            if card.base_damage_for(inst.upgraded) != 0 {
                v.push(CardEffect::Damage {
                    amount: card.base_damage_for(inst.upgraded),
                    hits: 1,
                });
            }
            if card.base_block_for(inst.upgraded) != 0 {
                v.push(CardEffect::Block {
                    amount: card.base_block_for(inst.upgraded),
                });
            }
            if card.apply_vulnerable_for(inst.upgraded) != 0 {
                v.push(CardEffect::ApplyVulnerable {
                    turns: card.apply_vulnerable_for(inst.upgraded),
                });
            }
            v
        } else {
            card.effects_for(inst.upgraded).to_vec()
        };

        for eff in effects {
            match eff {
                CardEffect::Damage { amount, hits } => {
                    let mut d = amount + next_state.player_strength + strength_this_turn;
                    if next_state.enemy_status.vulnerable.is_active() {
                        d = super::evaluate::apply_vulnerable_damage_multiplier(d);
                    }
                    damage += d * hits as i32;
                }
                CardEffect::Block { amount } => {
                    block += amount;
                }
                CardEffect::ApplyVulnerable { turns } => {
                    next_state.apply_vulnerable(turns);
                }
                CardEffect::Draw { count } => {
                    let mut drawn = draw_n(&mut next_state, count, rng);
                    hand.append(&mut drawn);
                }
                CardEffect::GainEnergy { amount } => {
                    energy += amount as i32;
                }
                CardEffect::GainStrengthThisTurn { amount } => {
                    strength_this_turn += amount;
                }
                CardEffect::GainStrength { amount } => {
                    next_state.player_strength += amount;
                }
                CardEffect::LoseHp { amount } => {
                    self_hp_loss += amount;
                }
                CardEffect::AddCopyToDiscard {
                    card_id,
                    upgraded,
                    count,
                } => {
                    // instance_idはユニークが望ましいが、ここでは「山札サイズ」影響が主なので簡易に採番
                    // draw_pile/discard_pile の最大+1から付与
                    let mut max_id = 0u32;
                    for c in next_state
                        .draw_pile
                        .iter()
                        .chain(next_state.discard_pile.iter())
                        .chain(hand.iter())
                    {
                        max_id = max_id.max(c.instance_id);
                    }
                    for i in 0..count {
                        next_state.discard_pile.push(CardInstance {
                            instance_id: max_id + 1 + i as u32,
                            card_id: card_id.clone(),
                            upgraded,
                        });
                    }
                }
                CardEffect::ExhaustSelf => {
                    next_state.exhaust_pile.push(inst.clone());
                }
                CardEffect::PutDiscardOnTop { count } => {
                    // ヒューリスティック: 捨て札の先頭からcount枚を山札上に戻す（本来は選択）
                    for _ in 0..count {
                        if let Some(c) = next_state.discard_pile.pop() {
                            next_state.draw_pile.push(c);
                        }
                    }
                }
                CardEffect::Unsupported { .. } => {}
            }
        }
    }

    // ターン終了: 手札とプレイ済みの未廃棄カードを捨て札へ
    next_state.discard_pile.extend(hand.into_iter());
    // playedのうち、ExhaustSelf を持つカードは exhaust_pile に入れたので discard へは入れない（簡易）
    // ただし ExhausSelf を effects で表現していないカードはここで捨て札に行く。
    // いずれ play.rs を使った正確な処理へ寄せる前提。
    next_state.discard_pile.extend(played.into_iter());

    next_state.tick_end_of_turn();

    let taken = (incoming_damage - block).max(0) + self_hp_loss.max(0);
    (next_state, damage as f64, block as f64, taken as f64)
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
        let (next_state, dmg, blk, taken) =
            simulate_one_turn_greedy(catalog, &state, incoming, rng);
        state = next_state;

        if t == 0 {
            turn1_damage = dmg;
            turn1_block = blk;
        }

        total_damage += dmg;
        total_block += blk;
        total_taken += taken;
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
