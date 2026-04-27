pub fn apply_vulnerable_damage_multiplier(base: i32) -> i32 {
    // Slay the Spire系: 脆弱は与ダメ1.5倍（端数切り捨て）
    (base * 3) / 2
}
