#!/usr/bin/env python3
# 日本語で応答するルールのため、出力やエラーも日本語に寄せる。

from __future__ import annotations

import argparse
import dataclasses
import datetime as dt
import json
import re
import sys
from typing import Any, Dict, List, Optional, Tuple

import requests
import yaml
from bs4 import BeautifulSoup


UNTAPPED_TIER_LIST = "https://sts2.untapped.gg/en/tier-list/cards/ironclad"
UNTAPPED_CARD_PREFIX = "https://sts2.untapped.gg/en/cards/"
WKWIKI_IRONCLAD_EARLY_ACCESS = (
    "https://wikiwiki.jp/sts2/%E3%82%A2%E3%82%A4%E3%82%A2%E3%83%B3%E3%82%AF%E3%83%A9%E3%83%83%E3%83%89%28%E3%82%A2%E3%83%BC%E3%83%AA%E3%83%BC%E3%82%A2%E3%82%AF%E3%82%BB%E3%82%B9%29"
)


@dataclasses.dataclass(frozen=True)
class UntappedCard:
    card_id: str
    name_en: str
    kind: str
    cost: str
    rarity: str
    text_base: str
    text_upgraded: Optional[str]


def http_get(url: str) -> str:
    resp = requests.get(url, timeout=30)
    resp.raise_for_status()
    return resp.text


def parse_untapped_card_links(html: str) -> List[str]:
    soup = BeautifulSoup(html, "html.parser")
    links: List[str] = []
    for a in soup.find_all("a", href=True):
        href = a["href"]
        if href.startswith("/en/cards/"):
            slug = href.split("/en/cards/")[1].split("?")[0].strip("/")
            if slug:
                links.append(slug)
    # 重複除去（順序維持）
    seen = set()
    out = []
    for s in links:
        if s in seen:
            continue
        seen.add(s)
        out.append(s)
    return out


def parse_untapped_card_page(card_id: str, html: str) -> UntappedCard:
    soup = BeautifulSoup(html, "html.parser")
    title = soup.find("title")
    title_text = (title.get_text() if title else "").strip()
    # 例: "Anger – Ironclad Common Attack – Slay the Spire 2 Card – Untapped.gg"
    name_en = title_text.split("–")[0].strip() if "–" in title_text else card_id

    def find_text_after(label: str) -> Optional[str]:
        # "TypeAttack" のように続くことがあるので、ページ内テキストから雑に抽出する
        full = soup.get_text("\n")
        m = re.search(rf"{re.escape(label)}\s*([A-Za-z0-9 +\-]+)", full)
        return m.group(1).strip() if m else None

    kind = (find_text_after("Type") or "Unknown").strip().lower()
    cost = (find_text_after("Cost") or "0").strip()
    rarity = (find_text_after("Rarity") or "Unknown").strip().lower()

    # 本文テキスト（Base/Upgraded の直下にある短い説明を拾う）
    full = soup.get_text("\n")
    # Base → 次の空行まで
    text_base = ""
    m_base = re.search(r"\nBase\s*\n+(.+?)\n\n", full, flags=re.DOTALL)
    if m_base:
        text_base = " ".join(m_base.group(1).split())
    # Upgraded
    text_upg = None
    m_upg = re.search(r"\nUPGRADED\s*\n+(.+?)\n\n", full, flags=re.DOTALL)
    if m_upg:
        text_upg = " ".join(m_upg.group(1).split())

    return UntappedCard(
        card_id=card_id,
        name_en=name_en,
        kind=kind,
        cost=cost,
        rarity=rarity,
        text_base=text_base,
        text_upgraded=text_upg,
    )


def normalize_kind(kind: str) -> str:
    k = kind.lower()
    if k == "attack":
        return "attack"
    if k == "skill":
        return "skill"
    if k == "power":
        return "power"
    if k == "curse":
        return "curse"
    return "skill"


def normalize_rarity(rarity: str) -> str:
    r = rarity.lower()
    if r in ("starter", "common", "uncommon", "rare", "ancient", "event", "shop", "special", "colorless", "status"):
        return r
    # Untappedは "Ironclad Common Attack" のような表現もあるため、部分一致を試す
    for key in ("starter", "common", "uncommon", "rare", "ancient"):
        if key in r:
            return key
    return "common"


def parse_cost(cost: str) -> int:
    if cost.upper() == "X":
        return 0
    m = re.search(r"\d+", cost)
    return int(m.group(0)) if m else 0


def effects_from_untapped_text(text: str, card_id: str) -> Tuple[List[Dict[str, Any]], Dict[str, int]]:
    """
    YAMLの CardDef 互換フィールド(base_damage/base_block/apply_vulnerable)も埋めたいので、
    (effects, legacy_fields) を返す。
    """
    effects: List[Dict[str, Any]] = []
    legacy = {"base_damage": 0, "base_block": 0, "apply_vulnerable": 0}

    # Deal N damage.
    m = re.search(r"Deal\s+(\d+)\s+damage", text, flags=re.IGNORECASE)
    if m:
        dmg = int(m.group(1))
        legacy["base_damage"] = dmg
        effects.append({"type": "damage", "amount": dmg, "hits": 1})

    # Gain N Block.
    m = re.search(r"Gain\s+(\d+)\s+Block", text, flags=re.IGNORECASE)
    if m:
        blk = int(m.group(1))
        legacy["base_block"] = blk
        effects.append({"type": "block", "amount": blk})

    # Apply N Vulnerable.
    m = re.search(r"Apply\s+(\d+)\s+Vulnerable", text, flags=re.IGNORECASE)
    if m:
        v = int(m.group(1))
        legacy["apply_vulnerable"] = v
        effects.append({"type": "apply_vulnerable", "turns": v})

    # Draw N cards.
    m = re.search(r"Draw\s+(\d+)\s+cards?", text, flags=re.IGNORECASE)
    if m:
        n = int(m.group(1))
        effects.append({"type": "draw", "count": n})

    # Lose N HP.
    m = re.search(r"Lose\s+(\d+)\s+HP", text, flags=re.IGNORECASE)
    if m:
        n = int(m.group(1))
        effects.append({"type": "lose_hp", "amount": n})

    # Add a copy of this card into your Discard Pile.
    if re.search(r"Add a copy of this card into your Discard Pile", text, flags=re.IGNORECASE):
        effects.append(
            {"type": "add_copy_to_discard", "card_id": card_id, "upgraded": False, "count": 1}
        )

    if not effects:
        effects.append({"type": "unsupported", "reason": f"未対応の効果テキスト: {text}"})

    return effects, legacy


def fetch_untapped_ironclad_cards() -> Tuple[List[UntappedCard], Dict[str, Any]]:
    tier_html = http_get(UNTAPPED_TIER_LIST)
    slugs = parse_untapped_card_links(tier_html)
    cards: List[UntappedCard] = []
    raw: Dict[str, Any] = {"tier_list_url": UNTAPPED_TIER_LIST, "cards": {}}

    for slug in slugs:
        url = UNTAPPED_CARD_PREFIX + slug
        try:
            html = http_get(url)
        except Exception as e:
            print(f"Untappedの取得に失敗: {url}: {e}", file=sys.stderr)
            continue
        c = parse_untapped_card_page(slug, html)
        cards.append(c)
        raw["cards"][slug] = {
            "url": url,
            "name_en": c.name_en,
            "kind": c.kind,
            "cost": c.cost,
            "rarity": c.rarity,
            "text_base": c.text_base,
            "text_upgraded": c.text_upgraded,
        }
    return cards, raw


def fetch_wkwiki_ironclad_cards() -> Dict[str, Any]:
    # 現状は「日本語名・効果テキストの参照用スナップショット」を主目的にする。
    # ID正規化が難しいため、YAML生成へのマージは段階的対応（今後）。
    html = http_get(WKWIKI_IRONCLAD_EARLY_ACCESS)
    soup = BeautifulSoup(html, "html.parser")
    text = soup.get_text("\n")
    return {
        "url": WKWIKI_IRONCLAD_EARLY_ACCESS,
        "text": text,
    }


def build_carddef_yaml_from_untapped(cards: List[UntappedCard]) -> List[Dict[str, Any]]:
    out: List[Dict[str, Any]] = []
    for c in cards:
        rarity = normalize_rarity(c.rarity)
        kind = normalize_kind(c.kind)
        base_effects, legacy_base = effects_from_untapped_text(c.text_base, c.card_id)

        upgraded_effects = None
        upgraded_legacy = {}
        if c.text_upgraded:
            upgraded_effects, upgraded_legacy = effects_from_untapped_text(c.text_upgraded, c.card_id)

        d: Dict[str, Any] = {
            "id": c.card_id,
            "name": c.name_en,
            "kind": kind,
            "rarity": rarity,
            "cost": parse_cost(c.cost),
            "base_damage": legacy_base["base_damage"],
            "base_block": legacy_base["base_block"],
            "apply_vulnerable": legacy_base["apply_vulnerable"],
            "text": c.text_base,
            "effects": base_effects,
        }

        if c.text_upgraded:
            # upgraded_* は「差分」ではなく「最終値」を入れる。未抽出はNone扱いでフォールバックする。
            d["upgraded_text"] = c.text_upgraded
            d["upgraded_effects"] = upgraded_effects
            if "base_damage" in upgraded_legacy:
                d["upgraded_base_damage"] = upgraded_legacy.get("base_damage")
            if "base_block" in upgraded_legacy:
                d["upgraded_base_block"] = upgraded_legacy.get("base_block")
            if "apply_vulnerable" in upgraded_legacy:
                d["upgraded_apply_vulnerable"] = upgraded_legacy.get("apply_vulnerable")
        out.append(d)
    return out


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--out", required=True, help="出力YAMLパス（例: ../../data/cards/ironclad.yaml）")
    ap.add_argument("--snapshot-out", required=True, help="スナップショットJSON出力パス")
    args = ap.parse_args()

    now = dt.datetime.now(dt.timezone.utc).isoformat()

    untapped_cards, untapped_raw = fetch_untapped_ironclad_cards()
    wkwiki_raw = fetch_wkwiki_ironclad_cards()

    defs = build_carddef_yaml_from_untapped(untapped_cards)
    with open(args.out, "w", encoding="utf-8") as f:
        yaml.safe_dump(defs, f, allow_unicode=True, sort_keys=False)

    snap = {
        "fetched_at_utc": now,
        "sources": {
            "untapped": untapped_raw,
            "wkwiki": wkwiki_raw,
        },
        "notes": [
            "YAML生成は現時点ではUntappedの英語テキストを主に使用しています。",
            "攻略Wikiは日本語テキストの参照用スナップショットとして保存しています（ID対応は今後拡張）。",
        ],
    }
    with open(args.snapshot_out, "w", encoding="utf-8") as f:
        json.dump(snap, f, ensure_ascii=False, indent=2)

    print(f"生成完了: {args.out}（{len(defs)}件）")
    print(f"スナップショット: {args.snapshot_out}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
