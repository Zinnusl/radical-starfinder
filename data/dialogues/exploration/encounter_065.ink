=== POWER_ROUTING ===
# id: 64
# title: The Power Routing Puzzle
# chinese_title: 电力迷局
# category: Puzzle
# mode: dungeon

A junction box controls power to three sections, but the routing diagram uses radical-based labels. You must connect the correct power lines to restore systems. Each connection is labeled with a radical component, and mismatches cause shorts.

* [Match the radicals to route power correctly]
  ~ gain_radical("雷")
  "电" (diàn) — electricity, power. Related to "雷" (léi) — thunder.
* [Trial and error with the connections]
  ~ damage(3)
  "错" (cuò) — wrong, mistake, error
* [Bypass the junction entirely] {requires: class == 1}
  ~ gain_xp(25)
  "通" (tōng) — to pass through, to connect
