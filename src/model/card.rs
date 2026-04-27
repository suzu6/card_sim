use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub type CardId = String;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CardRarity {
    Starter,
    Common,
    Uncommon,
    Rare,
    Ancient,
    Event,
    Shop,
    Special,
    Colorless,
    Status,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CardKind {
    Attack,
    Skill,
    Curse,
    Power,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum CardEffect {
    Damage { amount: i32, hits: u8 },
    Block { amount: i32 },
    ApplyVulnerable { turns: u8 },
    Draw { count: u8 },
    GainEnergy { amount: u8 },
    GainStrengthThisTurn { amount: i32 },
    GainStrength { amount: i32 },
    LoseHp { amount: i32 },
    AddCopyToDiscard { card_id: CardId, upgraded: bool, count: u8 },
    ExhaustSelf,
    PutDiscardOnTop { count: u8 },
    Unsupported { reason: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CardDef {
    pub id: CardId,
    pub name: String,
    pub kind: CardKind,
    #[serde(default)]
    pub rarity: Option<CardRarity>,
    pub cost: u8,
    pub base_damage: i32,
    pub base_block: i32,
    pub apply_vulnerable: u8,
    #[serde(default)]
    pub text: Option<String>,
    #[serde(default)]
    pub effects: Vec<CardEffect>,

    // 互換維持のため「アップグレード時の差分」もYAMLに持てるようにする。
    // 無指定の場合は非アップグレード値をそのまま使う。
    #[serde(default)]
    pub upgraded_cost: Option<u8>,
    #[serde(default)]
    pub upgraded_base_damage: Option<i32>,
    #[serde(default)]
    pub upgraded_base_block: Option<i32>,
    #[serde(default)]
    pub upgraded_apply_vulnerable: Option<u8>,
    #[serde(default)]
    pub upgraded_text: Option<String>,
    #[serde(default)]
    pub upgraded_effects: Option<Vec<CardEffect>>,
    pub draw: u8,
    #[serde(default)]
    pub add_block_on_attack_play: i32,
}

#[derive(Debug, Clone)]
pub struct CardCatalog {
    by_id: HashMap<CardId, CardDef>,
}

impl CardDef {
    pub fn cost_for(&self, upgraded: bool) -> u8 {
        if upgraded {
            self.upgraded_cost.unwrap_or(self.cost)
        } else {
            self.cost
        }
    }

    pub fn base_damage_for(&self, upgraded: bool) -> i32 {
        if upgraded {
            self.upgraded_base_damage.unwrap_or(self.base_damage)
        } else {
            self.base_damage
        }
    }

    pub fn base_block_for(&self, upgraded: bool) -> i32 {
        if upgraded {
            self.upgraded_base_block.unwrap_or(self.base_block)
        } else {
            self.base_block
        }
    }

    pub fn apply_vulnerable_for(&self, upgraded: bool) -> u8 {
        if upgraded {
            self.upgraded_apply_vulnerable
                .unwrap_or(self.apply_vulnerable)
        } else {
            self.apply_vulnerable
        }
    }

    pub fn effects_for(&self, upgraded: bool) -> &[CardEffect] {
        if upgraded {
            if let Some(e) = &self.upgraded_effects {
                return e;
            }
        }
        &self.effects
    }
}

impl CardCatalog {
    pub fn from_defs(defs: Vec<CardDef>) -> Self {
        let by_id = defs.into_iter().map(|d| (d.id.clone(), d)).collect();
        Self { by_id }
    }

    pub fn get(&self, id: &str) -> Option<&CardDef> {
        self.by_id.get(id)
    }

    pub fn iter(&self) -> impl Iterator<Item = (&CardId, &CardDef)> {
        self.by_id.iter()
    }
}
