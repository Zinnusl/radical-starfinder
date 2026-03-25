=== TOXIC_SPILL ===
# id: 37
# title: The Toxic Spill
# chinese_title: 有毒溢出
# category: Hazard
# mode: dungeon

Ruptured chemical storage tanks have flooded the lower deck with a viscous green fluid. The substance eats through standard materials but seems to pool around certain metals. Valuable equipment lies partially submerged in the toxic bath.

* [Salvage equipment using a long tool]
  ~ gain_gold(25)
  "拿" (ná) — to take, to grab
* [Neutralize the toxin with base compounds] {requires: class == 6}
  ~ gain_radical("水")
  "水" (shuǐ) — water, liquid
* [Carefully skirt the edges]
  ~ gain_xp(10)
  "边" (biān) — edge, side, border
* [Take a sample of the substance]
  ~ gain_item("toxin_grenade")
  "毒" (dú) — poison, toxic
