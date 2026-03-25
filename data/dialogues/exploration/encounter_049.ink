=== COOK_IN_HIDING ===
# id: 48
# title: The Hidden Cook
# chinese_title: 躲藏的厨师
# category: Crew
# mode: dungeon

The smell of actual cooking food stops you in your tracks. In a barricaded storage room, a station cook has set up a makeshift kitchen, using emergency fuel cells to heat salvaged ingredients. "Hungry?" she asks, stirring a pot with practiced ease.

* [Accept a hot meal gratefully]
  ~ heal(20)
  "吃" (chī) — to eat
* [Offer supplies to improve the meal] {requires: gold >= 10}
  ~ gain_radical("米")
  "米" (mǐ) — rice, food, grain
* [Ask for cooking lessons]
  ~ gain_xp(15)
  "做" (zuò) — to make, to do
* [Trade info about safe passages for food]
  ~ gain_item("stim_pack")
  "换" (huàn) — to exchange
