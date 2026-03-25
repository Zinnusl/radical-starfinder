=== ENGINE_TOMB ===
# id: 85
# title: The Engine Tomb
# chinese_title: 引擎之墓
# category: Wreckage
# mode: dungeon

A massive engine block, torn from its housing, has crashed through three decks. Coolant pools around its base, reflecting emergency lights in eerie blues. The engine's fusion core is cracked but not depleted — dangerous, but potentially valuable.

* [Carefully extract fusion core material] {requires: class == 1}
  ~ gain_radical("力")
  "核" (hé) — core, nuclear. The radical "力" (lì) means power.
* [Drain the remaining coolant for use]
  ~ gain_item("biogel_patch")
  "冷" (lěng) — cold, cool
* [Scavenge rare-metal components]
  ~ gain_gold(25)
  "铁" (tiě) — iron, metal
* [Mark the area as hazardous and leave]
  ~ gain_xp(10)
  "标" (biāo) — mark, sign, to label
