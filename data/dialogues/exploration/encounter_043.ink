=== BIOHAZARD_ZONE ===
# id: 42
# title: The Biohazard Zone
# chinese_title: 生化禁区
# category: Hazard
# mode: dungeon

Yellow biohazard markers line a quarantined section. Beyond the sealed doors, you can see growth — organic matter spreading across every surface. Your scanner detects unknown biological agents. Something valuable glints deep within the contaminated zone.

* [Don full hazmat gear and enter] {requires: hp >= 15}
  ~ gain_gold(35)
  "险" (xiǎn) — danger, risk
* [Sterilize a path with UV equipment] {requires: class == 6}
  ~ gain_radical("日")
  "灭" (miè) — to extinguish, to sterilize. The radical "日" (rì) means sun/light.
* [Seal the quarantine tighter]
  ~ gain_xp(15)
  "封" (fēng) — to seal
* [Send a drone to retrieve the object]
  ~ gain_item("scanner_pulse")
  "飞" (fēi) — to fly
