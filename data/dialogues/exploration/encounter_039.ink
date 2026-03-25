=== ELECTRICAL_STORM ===
# id: 38
# title: The Electrical Storm
# chinese_title: 电暴
# category: Hazard
# mode: dungeon

Cascading power failures have turned this section into a lightshow of deadly arcing electricity. Sparks leap between exposed conduits, and the air smells of ozone. Through the flickering chaos, you spot a shortcut to your destination.

* [Time your movement between the arcs]
  ~ gain_xp(20)
  "闪" (shǎn) — flash, to dodge
* [Shut down power to this section] {requires: class == 1}
  ~ gain_radical("雷")
  "雷" (léi) — thunder, lightning
* [Take the long way around]
  ~ nothing
  "远" (yuǎn) — far, distant
* [Absorb the energy with your equipment]
  ~ damage(6)
  "吸" (xī) — to absorb, to inhale
