use serde::{Deserialize, Serialize};

use super::CardId;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeckCardDef {
    pub id: CardId,
    pub count: u32,
    #[serde(default)]
    pub upgraded: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeckDef {
    pub cards: Vec<DeckCardDef>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Deck {
    pub cards: Vec<DeckCardDef>,
}

impl Deck {
    pub fn from_def(def: DeckDef) -> Self {
        Self { cards: def.cards }
    }

    pub fn add_one(&mut self, id: &str) {
        if let Some(c) = self.cards.iter_mut().find(|c| c.id == id && !c.upgraded) {
            c.count += 1;
            return;
        }
        self.cards.push(DeckCardDef {
            id: id.to_string(),
            count: 1,
            upgraded: false,
        });
    }

    pub fn remove_one(&mut self, id: &str) {
        if let Some(idx) = self.cards.iter().position(|c| c.id == id) {
            if self.cards[idx].count > 1 {
                self.cards[idx].count -= 1;
            } else {
                self.cards.remove(idx);
            }
        }
    }

    pub fn upgrade_one(&mut self, id: &str) {
        if let Some(idx) = self.cards.iter().position(|c| c.id == id) {
            if self.cards[idx].upgraded {
                return;
            }
            let base_count = self.cards[idx].count;
            if base_count == 0 {
                return;
            }
            if base_count > 1 {
                self.cards[idx].count -= 1;
            } else {
                self.cards.remove(idx);
            }
            self.cards.push(DeckCardDef {
                id: id.to_string(),
                count: 1,
                upgraded: true,
            });
        } else {
            self.cards.push(DeckCardDef {
                id: id.to_string(),
                count: 1,
                upgraded: true,
            });
        }
    }
}
