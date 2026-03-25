=== ALIEN_ANTIQUARIAN ===
# id: 21
# title: The Alien Antiquarian
# chinese_title: 异族古董商
# category: Trader
# mode: dungeon

A four-armed Kasatha sits behind a display of relics, each labeled in meticulous calligraphy. Their collection spans civilizations — pre-Gap artifacts, Drift-touched crystals, and objects that defy identification. They speak in measured, precise syllables.

* [Examine the pre-Gap artifacts] {requires: gold >= 30}
  ~ gain_radical("玉")
  "玉" (yù) — jade, precious stone
* [Inquire about the Drift-touched crystals]
  ~ gain_xp(20)
  "奇" (qí) — strange, rare, wonderful
* [Trade a personal item for store credit]
  ~ gain_gold(25)
  "古" (gǔ) — ancient, old
* [Commission an appraisal of something you found] {requires: gold >= 10}
  ~ gain_xp(15)
  "价" (jià) — price, value
