use crate::model::{CardCatalog, Deck};
use rand::{rngs::StdRng, seq::SliceRandom};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CardInstance {
    pub instance_id: u32,
    pub card_id: String,
    pub upgraded: bool,
}

pub fn build_draw_pile(deck: &Deck) -> Vec<CardInstance> {
    let mut next_id: u32 = 1;
    let mut out = vec![];
    for c in &deck.cards {
        for _ in 0..c.count {
            out.push(CardInstance {
                instance_id: next_id,
                card_id: c.id.clone(),
                upgraded: c.upgraded,
            });
            next_id += 1;
        }
    }
    out
}

pub fn validate_deck(deck: &Deck, catalog: &CardCatalog) -> Result<(), String> {
    for c in &deck.cards {
        if catalog.get(&c.id).is_none() {
            return Err(format!("デッキに未知のcard_idがあります: {}", c.id));
        }
    }
    Ok(())
}

#[allow(dead_code)]
fn n_choose_k(n: usize, k: usize) -> u128 {
    if k > n {
        return 0;
    }
    let k = k.min(n - k);
    let mut numer: u128 = 1;
    let mut denom: u128 = 1;
    for i in 0..k {
        numer *= (n - i) as u128;
        denom *= (i + 1) as u128;
    }
    numer / denom
}

#[allow(dead_code)]
fn combinations_indices(n: usize, k: usize) -> Vec<Vec<usize>> {
    fn rec(start: usize, n: usize, k: usize, cur: &mut Vec<usize>, out: &mut Vec<Vec<usize>>) {
        if cur.len() == k {
            out.push(cur.clone());
            return;
        }
        let remaining = k - cur.len();
        for i in start..=(n - remaining) {
            cur.push(i);
            rec(i + 1, n, k, cur, out);
            cur.pop();
        }
    }
    let mut out = vec![];
    let mut cur = vec![];
    if k == 0 {
        out.push(vec![]);
        return out;
    }
    if k > n {
        return out;
    }
    rec(0, n, k, &mut cur, &mut out);
    out
}

#[derive(Debug, Clone)]
pub struct DrawOutcome {
    pub hand: Vec<CardInstance>,
    pub new_draw_pile: Vec<CardInstance>,
    pub new_discard_pile: Vec<CardInstance>,
    pub probability: f64,
}

#[allow(dead_code)]
pub fn enumerate_draw_5(
    draw_pile: &[CardInstance],
    discard_pile: &[CardInstance],
) -> Vec<DrawOutcome> {
    enumerate_draw(draw_pile, discard_pile, 5)
}

#[allow(dead_code)]
pub fn enumerate_draw(
    draw_pile: &[CardInstance],
    discard_pile: &[CardInstance],
    n: usize,
) -> Vec<DrawOutcome> {
    let available = draw_pile.len();
    if available >= n {
        let denom = n_choose_k(available, n) as f64;
        let mut out = vec![];
        for idxs in combinations_indices(available, n) {
            let mut hand = vec![];
            let mut keep = vec![true; available];
            for &i in &idxs {
                hand.push(draw_pile[i].clone());
                keep[i] = false;
            }
            let new_draw_pile: Vec<_> = draw_pile
                .iter()
                .enumerate()
                .filter_map(|(i, c)| keep[i].then(|| c.clone()))
                .collect();
            out.push(DrawOutcome {
                hand,
                new_draw_pile,
                new_discard_pile: discard_pile.to_vec(),
                probability: 1.0 / denom,
            });
        }
        return out;
    }

    // 山札が足りない場合: 残りを全ドロー -> 捨て札をシャッフルした山札から残りをドロー
    let first_take = available;
    let second_take = n - first_take;
    let first_hand: Vec<CardInstance> = draw_pile.to_vec();

    let discard_n = discard_pile.len();
    let denom2 = n_choose_k(discard_n, second_take) as f64;
    let mut out = vec![];
    for idxs2 in combinations_indices(discard_n, second_take) {
        let mut hand = first_hand.clone();
        let mut keep2 = vec![true; discard_n];
        for &i in &idxs2 {
            hand.push(discard_pile[i].clone());
            keep2[i] = false;
        }
        let new_draw_pile: Vec<_> = discard_pile
            .iter()
            .enumerate()
            .filter_map(|(i, c)| keep2[i].then(|| c.clone()))
            .collect();
        out.push(DrawOutcome {
            hand,
            new_draw_pile,
            new_discard_pile: vec![],
            probability: 1.0 / denom2,
        });
    }
    out
}

pub fn draw_hand_random_5(
    draw_pile: &mut Vec<CardInstance>,
    discard_pile: &mut Vec<CardInstance>,
    rng: &mut StdRng,
) -> Vec<CardInstance> {
    draw_hand_random(draw_pile, discard_pile, 5, rng)
}

pub fn draw_hand_random_n(
    draw_pile: &mut Vec<CardInstance>,
    discard_pile: &mut Vec<CardInstance>,
    n: usize,
    rng: &mut StdRng,
) -> Vec<CardInstance> {
    draw_hand_random(draw_pile, discard_pile, n, rng)
}

fn draw_hand_random(
    draw_pile: &mut Vec<CardInstance>,
    discard_pile: &mut Vec<CardInstance>,
    n: usize,
    rng: &mut StdRng,
) -> Vec<CardInstance> {
    let mut hand = vec![];
    for _ in 0..n {
        if draw_pile.is_empty() {
            if discard_pile.is_empty() {
                break;
            }
            discard_pile.shuffle(rng);
            draw_pile.append(discard_pile);
        }
        let c = draw_pile.pop().expect("山札が空のはずがない");
        hand.push(c);
    }
    hand
}
