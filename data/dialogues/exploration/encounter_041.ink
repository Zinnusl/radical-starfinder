=== FIRE_CORRIDOR ===
# id: 40
# title: The Fire Corridor
# chinese_title: 火焰走廊
# category: Hazard
# mode: dungeon

A ruptured fuel line has turned the corridor into an inferno. Flames lick the walls and ceiling, the heat intense even through your suit. Through the fire, you can see a sealed door that might lead to safety — or to more danger.

* [Charge through the flames]
  ~ damage(8)
  "烈" (liè) — fierce, blazing, intense
* [Use fire suppression systems]
  ~ gain_radical("火")
  "火" (huǒ) — fire, flame
* [Find the fuel line shutoff] {requires: class == 1}
  ~ gain_xp(25)
  "阀" (fá) — valve, gate
* [Wait for the fire to burn itself out]
  ~ heal(5)
  "等" (děng) — to wait
