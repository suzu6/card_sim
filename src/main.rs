mod io;
mod model;
mod report;
mod sim;

use clap::Parser;
use io::{load_card_catalog, load_deck, load_enemy_pattern};
use model::{CardRarity, Deck};
use report::{compute_score, insight_from_deltas, render_rows_json, render_rows_table, render_rows_tsv, supported_level, ScoreWeights, SweepRow};
use sim::{simulate, SimulationInput};
use std::{fs, path::Path};
use time::{format_description::well_known::Rfc3339, OffsetDateTime};

#[derive(Debug, Clone, Copy)]
enum OutputFormat {
    Table,
    Tsv,
    Json,
}

fn parse_format(s: &str) -> Result<OutputFormat, String> {
    match s {
        "table" => Ok(OutputFormat::Table),
        "tsv" => Ok(OutputFormat::Tsv),
        "json" => Ok(OutputFormat::Json),
        _ => Err("formatは table|tsv|json から選んでください".to_string()),
    }
}

fn parse_score_weights(s: &str) -> Result<ScoreWeights, String> {
    // damage,taken,block の順（planに合わせる）
    let parts: Vec<&str> = s.split(',').collect();
    if parts.len() != 3 {
        return Err("score-weightsは damage,taken,block の3つをカンマ区切りで指定してください（例: 1,1,1）".to_string());
    }
    let damage: f64 = parts[0].trim().parse().map_err(|_| "damageの重みが数値ではありません".to_string())?;
    let taken: f64 = parts[1].trim().parse().map_err(|_| "takenの重みが数値ではありません".to_string())?;
    let block: f64 = parts[2].trim().parse().map_err(|_| "blockの重みが数値ではありません".to_string())?;
    Ok(ScoreWeights { damage, taken, block })
}

#[derive(Debug, Parser)]
#[command(name = "card_sim")]
#[command(about = "Slay the Spire 2 カードシミュレーション（最小MVP）", long_about = None)]
struct Args {
    /// カード定義YAML
    #[arg(long)]
    cards: String,

    /// デッキ定義YAML
    #[arg(long)]
    deck: String,

    /// 敵行動（固定パターン）YAML
    #[arg(long)]
    enemy: String,

    /// 評価ターン数（1 or 3想定）
    #[arg(long, default_value_t = 1)]
    turns: usize,

    /// モンテカルロの試行回数（大きいほど安定、遅くなる）
    #[arg(long, default_value_t = 50_000)]
    samples: u32,

    /// 乱数シード（固定すると再現性が上がる）
    #[arg(long, default_value_t = 1)]
    seed: u64,

    /// デッキにカードを1枚追加（card_id）
    #[arg(long)]
    add: Vec<String>,

    /// デッキからカードを1枚削除（card_id）
    #[arg(long)]
    remove: Option<String>,

    /// 指定カードをアップグレード扱いにする（card_id）
    #[arg(long)]
    upgrade: Option<String>,

    /// コモンを1枚追加した差分を全件一覧で出力する（アイアンクラッド想定）
    #[arg(long, default_value_t = false)]
    sweep_common: bool,

    /// sweepの出力形式（table|tsv|json）
    #[arg(long, default_value = "table", value_parser = parse_format)]
    format: OutputFormat,

    /// sweepの複合スコア重み（damage,taken,block）
    #[arg(long, default_value = "1,1,1", value_parser = parse_score_weights)]
    score_weights: ScoreWeights,

    /// 出力先ディレクトリ（sweep用）
    #[arg(long, default_value = "output")]
    out_dir: String,
}

fn apply_mutations(mut deck: Deck, args: &Args) -> Deck {
    for id in &args.add {
        deck.add_one(id);
    }
    if let Some(id) = &args.remove {
        deck.remove_one(id);
    }
    if let Some(id) = &args.upgrade {
        deck.upgrade_one(id);
    }
    deck
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    let catalog = load_card_catalog(&args.cards)?;
    let base_deck = load_deck(&args.deck)?;
    let enemy = load_enemy_pattern(&args.enemy)?;

    fn sanitize_for_filename(s: &str) -> String {
        s.chars()
            .map(|c| match c {
                'a'..='z' | 'A'..='Z' | '0'..='9' | '-' | '_' | '.' => c,
                _ => '_',
            })
            .collect()
    }

    fn basename(path: &str) -> String {
        Path::new(path)
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or(path)
            .to_string()
    }

    if args.sweep_common {
        let baseline_input = SimulationInput {
            catalog: &catalog,
            deck: &base_deck,
            enemy: &enemy,
            turns: args.turns,
            samples: args.samples,
            seed: args.seed,
        };
        let baseline = simulate(baseline_input)?;

        let mut rows: Vec<SweepRow> = vec![];
        for (card_id, def) in catalog.iter() {
            if def.rarity != Some(CardRarity::Common) {
                continue;
            }
            // まずは「1枚追加」のみ
            let mut deck = base_deck.clone();
            deck.add_one(card_id);
            let input = SimulationInput {
                catalog: &catalog,
                deck: &deck,
                enemy: &enemy,
                turns: args.turns,
                samples: args.samples,
                seed: args.seed,
            };
            let mutated = simulate(input)?;

            let delta_turn1_damage = mutated.turn1_expected_damage - baseline.turn1_expected_damage;
            let delta_turn1_block = mutated.turn1_expected_block - baseline.turn1_expected_block;
            let delta_total_damage = mutated.total_expected_damage - baseline.total_expected_damage;
            let delta_total_block = mutated.total_expected_block - baseline.total_expected_block;
            let delta_total_taken = mutated.total_expected_taken - baseline.total_expected_taken;

            let score = compute_score(
                args.score_weights,
                baseline.total_expected_damage,
                baseline.total_expected_block,
                baseline.total_expected_taken,
                delta_total_damage,
                delta_total_block,
                delta_total_taken,
            );

            rows.push(SweepRow {
                card_id: card_id.clone(),
                name: def.name.clone(),
                kind: def.kind,
                rarity: def.rarity,
                cost: def.cost,
                supported: supported_level(def),
                delta_turn1_damage,
                delta_turn1_block,
                delta_total_damage,
                delta_total_block,
                delta_total_taken,
                score,
                insight: insight_from_deltas(delta_total_damage, delta_total_block, delta_total_taken),
            });
        }

        rows.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));

        // Markdownとしてoutput/に保存
        let now = OffsetDateTime::now_utc();
        let ts = now
            .format(&Rfc3339)
            .unwrap_or_else(|_| "unknown_time".to_string())
            .replace(':', "")
            .replace('-', "")
            .replace('T', "_")
            .replace('Z', "Z");

        let enemy_name = sanitize_for_filename(&basename(&args.enemy).replace(".yaml", ""));
        let deck_name = sanitize_for_filename(&basename(&args.deck).replace(".yaml", ""));
        let cards_name = sanitize_for_filename(&basename(&args.cards).replace(".yaml", ""));
        let weights = format!(
            "{}-{}-{}",
            args.score_weights.damage, args.score_weights.taken, args.score_weights.block
        );
        let weights = sanitize_for_filename(&weights);

        let filename = format!(
            "sweep_common_{ts}_turns{turns}_samples{samples}_seed{seed}_enemy{enemy}_deck{deck}_cards{cards}_w{w}.md",
            ts = ts,
            turns = args.turns,
            samples = args.samples,
            seed = args.seed,
            enemy = enemy_name,
            deck = deck_name,
            cards = cards_name,
            w = weights
        );
        fs::create_dir_all(&args.out_dir)?;
        let out_path = Path::new(&args.out_dir).join(filename);

        let mut md = String::new();
        md.push_str("## sweep-common 結果\n\n");
        md.push_str("### 条件\n");
        md.push_str(&format!("- **cards**: `{}`\n", args.cards));
        md.push_str(&format!("- **deck**: `{}`\n", args.deck));
        md.push_str(&format!("- **enemy**: `{}`\n", args.enemy));
        md.push_str(&format!("- **turns**: `{}`\n", args.turns));
        md.push_str(&format!("- **samples**: `{}`\n", args.samples));
        md.push_str(&format!("- **seed**: `{}`\n", args.seed));
        md.push_str(&format!(
            "- **score_weights (damage,taken,block)**: `{},{},{}`\n",
            args.score_weights.damage, args.score_weights.taken, args.score_weights.block
        ));
        md.push_str("\n### baseline\n");
        md.push_str(&format!(
            "- turn1_expected_damage: `{:.6}`\n- turn1_expected_block: `{:.6}`\n- total_expected_damage: `{:.6}`\n- total_expected_block: `{:.6}`\n- total_expected_taken: `{:.6}`\n",
            baseline.turn1_expected_damage,
            baseline.turn1_expected_block,
            baseline.total_expected_damage,
            baseline.total_expected_block,
            baseline.total_expected_taken
        ));
        md.push('\n');

        md.push_str("### 一覧（score降順）\n\n");
        match args.format {
            OutputFormat::Table => {
                md.push_str(&render_rows_table(&rows));
            }
            OutputFormat::Tsv => {
                md.push_str("```tsv\n");
                md.push_str(&render_rows_tsv(&rows));
                md.push_str("```\n");
            }
            OutputFormat::Json => {
                md.push_str("```json\n");
                md.push_str(&render_rows_json(&rows)?);
                md.push_str("\n```\n");
            }
        }

        fs::write(&out_path, md)?;
        println!("{}", out_path.display());

        return Ok(());
    }

    let deck = apply_mutations(base_deck.clone(), &args);
    let input = SimulationInput {
        catalog: &catalog,
        deck: &deck,
        enemy: &enemy,
        turns: args.turns,
        samples: args.samples,
        seed: args.seed,
    };

    let baseline_input = SimulationInput {
        catalog: &catalog,
        deck: &base_deck,
        enemy: &enemy,
        turns: args.turns,
        samples: args.samples,
        seed: args.seed,
    };

    let baseline = simulate(baseline_input)?;
    let mutated = simulate(input)?;

    println!("== baseline ==");
    println!(
        "turn1_expected_damage: {:.6}\nturn1_expected_block: {:.6}",
        baseline.turn1_expected_damage, baseline.turn1_expected_block
    );
    println!(
        "turn{}_expected_damage_total: {:.6}\nturn{}_expected_block_total: {:.6}\nturn{}_expected_taken_total: {:.6}",
        args.turns,
        baseline.total_expected_damage,
        args.turns,
        baseline.total_expected_block,
        args.turns,
        baseline.total_expected_taken
    );

    println!("\n== mutated ==");
    println!(
        "turn1_expected_damage: {:.6}\nturn1_expected_block: {:.6}",
        mutated.turn1_expected_damage, mutated.turn1_expected_block
    );
    println!(
        "turn{}_expected_damage_total: {:.6}\nturn{}_expected_block_total: {:.6}\nturn{}_expected_taken_total: {:.6}",
        args.turns,
        mutated.total_expected_damage,
        args.turns,
        mutated.total_expected_block,
        args.turns,
        mutated.total_expected_taken
    );

    println!("\n== delta (mutated - baseline) ==");
    println!(
        "turn1_damage: {:+.6}\nturn1_block: {:+.6}\nturn{}_damage_total: {:+.6}\nturn{}_block_total: {:+.6}\nturn{}_taken_total: {:+.6}",
        mutated.turn1_expected_damage - baseline.turn1_expected_damage,
        mutated.turn1_expected_block - baseline.turn1_expected_block,
        args.turns,
        mutated.total_expected_damage - baseline.total_expected_damage,
        args.turns,
        mutated.total_expected_block - baseline.total_expected_block,
        args.turns,
        mutated.total_expected_taken - baseline.total_expected_taken
    );

    Ok(())
}
