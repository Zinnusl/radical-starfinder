=== MERCENARY_BAND ===
# id: 52
# title: The Mercenary Band
# chinese_title: 佣兵团
# category: Crew
# mode: dungeon

Three armed mercenaries block the corridor, their leader stepping forward with a professional smile. "We're offering protection services," she says. "This sector's dangerous. For a modest fee, we'll make sure nothing bothers you for a while."

* [Hire them for protection] {requires: gold >= 25}
  ~ gain_radical("刀")
  "兵" (bīng) — soldier, weapon. The radical "刀" (dāo) means blade/knife.
* [Negotiate a lower price]
  ~ gain_xp(15)
  "谈" (tán) — to talk, to negotiate
* [Decline — you can handle yourself]
  ~ gain_xp(10)
  "强" (qiáng) — strong, powerful
* [Challenge the leader to prove their worth]
  ~ start_fight
  "打" (dǎ) — to fight, to hit
