=== FROZEN_GARDEN ===
# id: 3
# title: The Frozen Garden
# chinese_title: 冰园
# category: Discovery
# mode: dungeon

Behind a frosted airlock, you discover an abandoned biodome. Crystallized plants stand frozen mid-bloom, their leaves encased in shimmering ice. A malfunctioning climate system has turned this garden into a sculpture of preserved life. Some specimens look valuable to xenobiologists.

* [Collect frozen plant samples]
  ~ gain_radical("木")
  "木" (mù) — wood, tree, plant
* [Repair the climate controls] {requires: class == 1}
  ~ gain_xp(25)
  "修" (xiū) — to repair, to fix
* [Melt the ice carefully with a heat source]
  ~ gain_item("biogel_patch")
  "冰" (bīng) — ice
