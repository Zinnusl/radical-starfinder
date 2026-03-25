=== MEDICAL_BAY_WRECK ===
# id: 89
# title: The Wrecked Medical Bay
# chinese_title: 毁坏医疗舱
# category: Wreckage
# mode: dungeon

The medical bay has been torn open by structural failure. Surgical robots hang frozen mid-procedure, their patients long gone. Medical supplies have scattered across the tilted floor, many still sealed in protective packaging. A pharmaceutical cabinet remains locked.

* [Gather the scattered medical supplies]
  ~ gain_item("med_hypo")
  "药" (yào) — medicine, drug
* [Hack the pharmaceutical cabinet] {requires: class == 3}
  ~ gain_radical("竹")
  "竹" (zhú) — bamboo. Traditional medicine containers were made of bamboo.
* [Salvage surgical robot components]
  ~ gain_gold(20)
  "机" (jī) — machine, mechanism
* [Look for patient records]
  ~ gain_xp(15)
  "病" (bìng) — illness, disease
