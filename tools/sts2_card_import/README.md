# STS2 カード取り込みツール（アイアンクラッド）

外部サイトの公開情報（Untapped.gg / Slay the Spire 2攻略 Wiki*）からカード情報を取得し、`card_sim` が読み込めるYAMLへ変換します。

## 使い方

```bash
python3 -m venv .venv
. .venv/bin/activate
pip install -r requirements.txt

python import_ironclad.py \
  --out ../../data/cards/ironclad.yaml \
  --snapshot-out ../../data/sources/sts2_cards_snapshot.json
```

## 出力

- `data/cards/ironclad.yaml`: `Vec<CardDef>` として読み込めるカード定義
- `data/sources/sts2_cards_snapshot.json`: 取得元・取得日時・生テキスト等のスナップショット（差分確認用）

## 注意

- 本ゲームはアーリーアクセスのため、カード性能は更新で変わります。スナップショットの更新は定期的に行ってください。
