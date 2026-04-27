mod io;
mod model;
mod sim;

use clap::Parser;
use io::{load_card_catalog, load_deck, load_enemy_pattern};
use model::Deck;
use sim::{simulate, SimulationInput};

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
    add: Option<String>,

    /// デッキからカードを1枚削除（card_id）
    #[arg(long)]
    remove: Option<String>,

    /// 指定カードをアップグレード扱いにする（card_id）
    #[arg(long)]
    upgrade: Option<String>,
}

fn apply_mutations(mut deck: Deck, args: &Args) -> Deck {
    if let Some(id) = &args.add {
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
