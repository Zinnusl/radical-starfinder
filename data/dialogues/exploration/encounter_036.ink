=== RADIATION_LEAK ===
# id: 35
# title: The Radiation Leak
# chinese_title: 辐射泄漏
# category: Hazard
# mode: dungeon

Warning klaxons blare as your suit detects elevated radiation levels. A cracked reactor shielding panel leaks ionizing particles into the corridor. The path forward cuts through the danger zone, but there may be another way around through maintenance tunnels.

* [Sprint through the radiation zone]
  ~ damage(10)
  "跑" (pǎo) — to run, to sprint
* [Find the maintenance tunnel route]
  ~ gain_xp(15)
  "路" (lù) — road, path, route
* [Attempt to seal the leak] {requires: class == 1}
  ~ gain_radical("气")
  "气" (qì) — air, gas, energy
* [Use a nano-shield for protection] {requires: radical == "金"}
  ~ gain_item("nano_shield")
  "防" (fáng) — defense, to prevent
