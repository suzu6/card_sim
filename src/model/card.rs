use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub type CardId = String;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CardKind {
    Attack,
    Skill,
    Curse,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CardDef {
    pub id: CardId,
    pub name: String,
    pub kind: CardKind,
    pub cost: u8,
    pub base_damage: i32,
    pub base_block: i32,
    pub apply_vulnerable: u8,
}

#[derive(Debug, Clone)]
pub struct CardCatalog {
    by_id: HashMap<CardId, CardDef>,
}

impl CardCatalog {
    pub fn from_defs(defs: Vec<CardDef>) -> Self {
        let by_id = defs.into_iter().map(|d| (d.id.clone(), d)).collect();
        Self { by_id }
    }

    pub fn get(&self, id: &str) -> Option<&CardDef> {
        self.by_id.get(id)
    }
}
