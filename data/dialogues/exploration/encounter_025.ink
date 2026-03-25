=== FORTUNE_SELLER ===
# id: 24
# title: The Fortune Seller
# chinese_title: 卜卦师
# category: Trader
# mode: dungeon

An elderly figure sits beneath a canopy of holographic stars, their fingers tracing patterns on a divination tablet. "Your future is written in the radicals," they whisper. "Each stroke holds meaning, each character a destiny." They offer to read your fortune for a small fee.

* [Pay for a fortune reading] {requires: gold >= 10}
  ~ gain_radical("大")
  "大" (dà) — big, great, grand
* [Ask about the divination tablet's origin]
  ~ gain_xp(20)
  "命" (mìng) — fate, destiny, life
* [Challenge them to prove their abilities]
  ~ gain_item("focus_stim")
  "试" (shì) — to try, to test
* [Offer to trade knowledge instead]
  ~ gain_radical("小")
  "小" (xiǎo) — small, little
