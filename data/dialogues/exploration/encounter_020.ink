=== DATA_BROKER ===
# id: 19
# title: The Data Broker
# chinese_title: 数据商
# category: Trader
# mode: dungeon

A holographic avatar materializes from a concealed projector — the calling card of a data broker. The featureless figure offers information: station layouts, patrol schedules, hidden caches. Every piece of intel has a price, displayed in floating numerals.

* [Buy station schematics] {requires: gold >= 20}
  ~ gain_xp(30)
  "图" (tú) — map, diagram, picture
* [Purchase cache locations] {requires: gold >= 25}
  ~ lose_gold(25)
  "位" (wèi) — position, location
* [Sell your own survey data]
  ~ gain_gold(15)
  "数" (shù) — number, data
* [Try to trace the broker's signal] {requires: class == 6}
  ~ gain_radical("门")
  "门" (mén) — door, gate, entrance
