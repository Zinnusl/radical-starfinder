=== RIVAL_EXPLORER ===
# id: 47
# title: The Rival Explorer
# chinese_title: 对手探险家
# category: Crew
# mode: dungeon

Another explorer rounds the corner, weapon drawn. Recognition flashes across their face — you've crossed paths before. They lower their weapon slowly. "Same target?" they ask, nodding toward the restricted section ahead. "We could work together. Or not."

* [Agree to cooperate and split the rewards]
  ~ gain_radical("又")
  "又" (yòu) — again, also, both
* [Challenge them to a race for the prize]
  ~ gain_xp(20)
  "赛" (sài) — competition, race
* [Offer to buy them out] {requires: gold >= 30}
  ~ lose_gold(30)
  "合" (hé) — to combine, together
* [Refuse and go your own way]
  ~ gain_xp(10)
  "独" (dú) — alone, independent
