# card_sim

Slay the Spire 2 のカードシミュレーション（最小MVP）です。

## 実行方法

### 通常のシミュレーション（差分ではなく単発）

```bash
cargo run -q -- \
  --cards data/cards/ironclad.yaml \
  --deck data/decks/ironclad_starter.yaml \
  --enemy data/enemies/dummy_3turn.yaml \
  --turns 3 \
  --samples 50000 \
  --seed 1
```

### sweep（カードを1枚追加した差分を一覧化）

指定したレアリティのカードを「デッキに1枚追加」した場合の期待値差分を総当たりし、`output/` にレポートを出力します。

- 出力ファイル
  - Markdown: `output/sweep_*.md`
  - CSV: `output/sweep_*.csv`（表結果をCSVでも保存）
- `--format` は Markdown 内に埋め込む表の形式です（`table|tsv|json`）。CSVは `--format` に関係なく常に保存します。

#### コモンだけ

```bash
cargo run -q -- \
  --cards data/cards/ironclad.yaml \
  --deck data/decks/ironclad_starter.yaml \
  --enemy data/enemies/dummy_3turn.yaml \
  --turns 3 \
  --samples 500 \
  --seed 1 \
  --sweep-common \
  --format table
```

#### アンコモンだけ / レアだけ

```bash
# アンコモン
cargo run -q -- \
  --cards data/cards/ironclad.yaml \
  --deck data/decks/ironclad_starter.yaml \
  --enemy data/enemies/dummy_3turn.yaml \
  --turns 3 \
  --samples 500 \
  --seed 1 \
  --sweep-uncommon \
  --format table

# レア
cargo run -q -- \
  --cards data/cards/ironclad.yaml \
  --deck data/decks/ironclad_starter.yaml \
  --enemy data/enemies/dummy_3turn.yaml \
  --turns 3 \
  --samples 500 \
  --seed 1 \
  --sweep-rare \
  --format table
```

#### 全部ひとまとめ（コモン+アンコモン+レア）

```bash
cargo run -q -- \
  --cards data/cards/ironclad.yaml \
  --deck data/decks/ironclad_starter.yaml \
  --enemy data/enemies/dummy_3turn.yaml \
  --turns 3 \
  --samples 500 \
  --seed 1 \
  --sweep-all \
  --format table
```

#### 任意の組み合わせをひとまとめ（例: コモン+レア）

```bash
cargo run -q -- \
  --cards data/cards/ironclad.yaml \
  --deck data/decks/ironclad_starter.yaml \
  --enemy data/enemies/dummy_3turn.yaml \
  --turns 3 \
  --samples 500 \
  --seed 1 \
  --sweep-common \
  --sweep-rare \
  --format table
```
