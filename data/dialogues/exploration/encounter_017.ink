=== BLACK_MARKET_NODE ===
# id: 16
# title: The Black Market Node
# chinese_title: 黑市
# category: Trader
# mode: dungeon

A hidden alcove behind a false wall panel reveals a small automated trading terminal. Its interface is crude but functional, listing items at steep prices. A warning flashes: unauthorized commerce detected. Someone has hacked the station's commerce system to run this shadow market.

* [Buy discounted stim packs] {requires: gold >= 15}
  ~ gain_item("stim_pack")
  "黑" (hēi) — black, dark
* [Hack the terminal for free goods] {requires: class == 3}
  ~ gain_gold(30)
  "偷" (tōu) — to steal
* [Report the terminal]
  ~ gain_xp(20)
  "法" (fǎ) — law, method
* [Sell salvage for quick credits]
  ~ gain_gold(20)
  "卖" (mài) — to sell
