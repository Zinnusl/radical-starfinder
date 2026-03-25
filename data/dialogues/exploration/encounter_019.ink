=== MEDICINE_PEDDLER ===
# id: 18
# title: The Medicine Peddler
# chinese_title: 药贩
# category: Trader
# mode: dungeon

A nervous-looking woman operates a mobile medical stall from a converted cargo drone. Her remedies range from standard-issue biogel to suspiciously unlabeled vials. She claims everything is genuine military surplus, but her darting eyes suggest otherwise.

* [Buy standard medical supplies] {requires: gold >= 10}
  ~ gain_item("med_hypo")
  "医" (yī) — medicine, to heal
* [Purchase the unlabeled vials] {requires: gold >= 25}
  ~ gain_item("focus_stim")
  "毒" (dú) — poison, toxic
* [Ask for a free sample]
  ~ heal(8)
  "免" (miǎn) — free, exempt
* [Barter your own supplies]
  ~ gain_radical("米")
  "米" (mǐ) — rice, grain
