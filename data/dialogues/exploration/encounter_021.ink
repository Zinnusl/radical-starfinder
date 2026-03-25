=== REFUGEE_TRADER ===
# id: 20
# title: The Refugee Trader
# chinese_title: 难民商人
# category: Trader
# mode: dungeon

A family of refugees has set up a makeshift stall near the docking bay. Their goods are modest — hand-sewn patches, recycled components, home-cooked rations — but their craftsmanship is excellent. A child tugs at your sleeve, offering a polished stone.

* [Buy the polished stone from the child] {requires: gold >= 5}
  ~ gain_radical("子")
  "子" (zǐ) — child, son
* [Purchase their best crafted item] {requires: gold >= 15}
  ~ gain_equipment
  "家" (jiā) — family, home
* [Donate credits without buying anything] {requires: gold >= 20}
  ~ gain_xp(25)
  "善" (shàn) — good, kind, charity
* [Trade stories over their home-cooked food]
  ~ heal(10)
  "食" (shí) — food, to eat
