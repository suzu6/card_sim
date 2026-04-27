#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Vulnerable {
    pub turns: u8,
}

impl Vulnerable {
    pub fn new(turns: u8) -> Self {
        Self { turns }
    }

    pub fn is_active(&self) -> bool {
        self.turns > 0
    }

    pub fn tick_end_of_turn(&mut self) {
        self.turns = self.turns.saturating_sub(1);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EnemyStatus {
    pub vulnerable: Vulnerable,
}

impl EnemyStatus {
    pub fn new() -> Self {
        Self {
            vulnerable: Vulnerable::new(0),
        }
    }
}
