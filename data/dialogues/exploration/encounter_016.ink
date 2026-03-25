=== WANDERING_MERCHANT ===
# id: 15
# title: The Wandering Merchant
# chinese_title: 行商
# category: Trader
# mode: dungeon

A cloaked figure sits cross-legged beside a hover-cart laden with curiosities. She greets you with a nod, her cybernetic eye whirring as it scans you. Her wares range from mundane rations to objects that shimmer with unexplained energy. A hand-painted sign reads "Fair Trades Only" in six languages.

* [Browse her exotic wares] {requires: gold >= 20}
  ~ gain_radical("贝")
  "贝" (bèi) — shell, currency, value
* [Trade information instead of credits]
  ~ gain_xp(15)
  "换" (huàn) — to exchange, to trade
* [Ask about rare components]
  ~ gain_item("neural_boost")
  "买" (mǎi) — to buy
* [Move on without stopping]
  ~ nothing
  "走" (zǒu) — to walk, to leave
