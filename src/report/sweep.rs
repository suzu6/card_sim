use crate::model::{CardDef, CardEffect, CardKind, CardRarity};
use serde::Serialize;

#[derive(Debug, Clone, Copy)]
pub struct ScoreWeights {
    pub damage: f64,
    pub block: f64,
    pub taken: f64,
}

impl ScoreWeights {
    // clapのdefault_valueで構築するため、ここではデフォルト関数は持たない
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SupportedLevel {
    Supported,
    Partial,
    Unsupported,
}

#[derive(Debug, Clone, Serialize)]
pub struct SweepRow {
    pub card_id: String,
    pub name: String,
    pub name_ja: Option<String>,
    pub kind: CardKind,
    pub rarity: Option<CardRarity>,
    pub cost: u8,
    pub supported: SupportedLevel,

    pub delta_turn1_damage: f64,
    pub delta_turn1_block: f64,
    pub delta_total_damage: f64,
    pub delta_total_block: f64,
    pub delta_total_taken: f64,

    pub score: f64,
    pub insight: String,
}

fn has_unsupported_effect(def: &CardDef) -> bool {
    def.effects
        .iter()
        .any(|e| matches!(e, CardEffect::Unsupported { .. }))
        || def
            .upgraded_effects
            .as_ref()
            .map(|v| v.iter().any(|e| matches!(e, CardEffect::Unsupported { .. })))
            .unwrap_or(false)
}

fn is_effect_supported_now(e: &CardEffect) -> bool {
    matches!(
        e,
        CardEffect::Damage { .. }
            | CardEffect::Block { .. }
            | CardEffect::ApplyVulnerable { .. }
            | CardEffect::Draw { .. }
            | CardEffect::GainEnergy { .. }
            | CardEffect::GainStrengthThisTurn { .. }
            | CardEffect::GainStrength { .. }
            | CardEffect::LoseHp { .. }
            | CardEffect::AddCopyToDiscard { .. }
            | CardEffect::ExhaustSelf
            | CardEffect::PutDiscardOnTop { .. }
    )
}

pub fn supported_level(def: &CardDef) -> SupportedLevel {
    if has_unsupported_effect(def) {
        return SupportedLevel::Unsupported;
    }

    let base_ok = def.effects.iter().all(is_effect_supported_now);
    let upg_ok = def
        .upgraded_effects
        .as_ref()
        .map(|v| v.iter().all(is_effect_supported_now))
        .unwrap_or(true);

    if base_ok && upg_ok {
        SupportedLevel::Supported
    } else {
        SupportedLevel::Partial
    }
}

pub fn compute_score(
    weights: ScoreWeights,
    baseline_total_damage: f64,
    baseline_total_block: f64,
    baseline_total_taken: f64,
    delta_total_damage: f64,
    delta_total_block: f64,
    delta_total_taken: f64,
) -> f64 {
    let norm_damage = delta_total_damage / baseline_total_damage.abs().max(1.0);
    let norm_block = delta_total_block / baseline_total_block.abs().max(1.0);
    let norm_taken = -delta_total_taken / baseline_total_taken.abs().max(1.0);
    weights.damage * norm_damage + weights.block * norm_block + weights.taken * norm_taken
}

pub fn insight_from_deltas(delta_damage: f64, delta_block: f64, delta_taken: f64) -> String {
    // 高精度な分類ではなく「どこに効いたか」の目安を出す
    let mut tags: Vec<&str> = vec![];
    if delta_taken < 0.0 {
        tags.push("被ダメ改善");
    }
    if delta_damage > 0.0 {
        tags.push("火力改善");
    }
    if delta_block > 0.0 {
        tags.push("防御改善");
    }
    if tags.is_empty() {
        "変化小/悪化".to_string()
    } else {
        tags.join("+")
    }
}

pub fn render_rows_table(rows: &[SweepRow]) -> String {
    // 固定幅の簡易テーブル（日本語でも崩れにくいように区切りはタブを避ける）
    let mut out = String::new();
    out.push_str("rank | card_id | name | name_ja | rarity | cost | kind | supported | Δdmg | Δblk | Δtaken | score | insight\n");
    out.push_str("-----|---------|------|---------|--------|------|------|-----------|------|------|--------|-------|--------\n");
    for (i, r) in rows.iter().enumerate() {
        let rarity = r
            .rarity
            .map(|v| format!("{v:?}"))
            .unwrap_or_else(|| "-".to_string());
        let name_ja = r.name_ja.clone().unwrap_or_else(|| "-".to_string());
        out.push_str(&format!(
            "{rank} | {id} | {name} | {name_ja} | {rarity} | {cost} | {kind:?} | {supp:?} | {dd:+.3} | {db:+.3} | {dt:+.3} | {score:+.4} | {ins}\n",
            rank = i + 1,
            id = r.card_id,
            name = r.name,
            name_ja = name_ja,
            rarity = rarity,
            cost = r.cost,
            kind = r.kind,
            supp = r.supported,
            dd = r.delta_total_damage,
            db = r.delta_total_block,
            dt = r.delta_total_taken,
            score = r.score,
            ins = r.insight
        ));
    }
    out
}

pub fn render_rows_tsv(rows: &[SweepRow]) -> String {
    let mut out = String::new();
    out.push_str("rank\tcard_id\tname\tname_ja\trarity\tcost\tkind\tsupported\tdelta_turn1_damage\tdelta_turn1_block\tdelta_total_damage\tdelta_total_block\tdelta_total_taken\tscore\tinsight\n");
    for (i, r) in rows.iter().enumerate() {
        let rarity = r
            .rarity
            .map(|v| format!("{v:?}"))
            .unwrap_or_else(|| "-".to_string());
        let name_ja = r.name_ja.clone().unwrap_or_default();
        out.push_str(&format!(
            "{rank}\t{id}\t{name}\t{name_ja}\t{rarity}\t{cost}\t{kind:?}\t{supp:?}\t{t1d:+.6}\t{t1b:+.6}\t{td:+.6}\t{tb:+.6}\t{tt:+.6}\t{score:+.6}\t{ins}\n",
            rank = i + 1,
            id = r.card_id,
            name = r.name.replace('\t', " "),
            name_ja = name_ja.replace('\t', " "),
            rarity = rarity.replace('\t', " "),
            cost = r.cost,
            kind = r.kind,
            supp = r.supported,
            t1d = r.delta_turn1_damage,
            t1b = r.delta_turn1_block,
            td = r.delta_total_damage,
            tb = r.delta_total_block,
            tt = r.delta_total_taken,
            score = r.score,
            ins = r.insight.replace('\t', " "),
        ));
    }
    out
}

fn escape_csv_field(s: &str) -> String {
    // RFC4180相当:
    // - フィールドに , " 改行 が含まれる場合は全体を "..." で囲む
    // - " は "" にエスケープする
    let must_quote = s.contains(',') || s.contains('"') || s.contains('\n') || s.contains('\r');
    if !must_quote {
        return s.to_string();
    }
    let escaped = s.replace('"', "\"\"");
    format!("\"{escaped}\"")
}

pub fn render_rows_csv(rows: &[SweepRow]) -> String {
    let mut out = String::new();
    out.push_str("rank,card_id,name,name_ja,rarity,cost,kind,supported,delta_turn1_damage,delta_turn1_block,delta_total_damage,delta_total_block,delta_total_taken,score,insight\n");
    for (i, r) in rows.iter().enumerate() {
        let name = escape_csv_field(&r.name);
        let name_ja = escape_csv_field(r.name_ja.as_deref().unwrap_or(""));
        let rarity = r
            .rarity
            .map(|v| format!("{v:?}"))
            .unwrap_or_else(|| "-".to_string());
        let rarity = escape_csv_field(&rarity);
        let insight = escape_csv_field(&r.insight);
        out.push_str(&format!(
            "{rank},{id},{name},{name_ja},{rarity},{cost},{kind:?},{supp:?},{t1d:+.6},{t1b:+.6},{td:+.6},{tb:+.6},{tt:+.6},{score:+.6},{insight}\n",
            rank = i + 1,
            id = r.card_id,
            name = name,
            name_ja = name_ja,
            rarity = rarity,
            cost = r.cost,
            kind = r.kind,
            supp = r.supported,
            t1d = r.delta_turn1_damage,
            t1b = r.delta_turn1_block,
            td = r.delta_total_damage,
            tb = r.delta_total_block,
            tt = r.delta_total_taken,
            score = r.score,
            insight = insight,
        ));
    }
    out
}

pub fn render_rows_json(rows: &[SweepRow]) -> Result<String, serde_json::Error> {
    serde_json::to_string_pretty(rows)
}
