=== MYSTIC_WANDERER ===
# id: 50
# title: The Mystic Wanderer
# chinese_title: 神秘流浪者
# category: Crew
# mode: dungeon

A robed figure meditates in the center of an empty cargo bay, surrounded by floating crystals arranged in geometric patterns. They open one eye as you approach. "The universe speaks in radicals," they say. "Would you like to listen?"

* [Sit and meditate with the mystic]
  ~ gain_radical("心")
  "静" (jìng) — quiet, still, calm. The radical "心" (xīn) means heart/mind.
* [Ask them to teach you] {requires: class == 2}
  ~ gain_xp(30)
  "教" (jiāo) — to teach
* [Offer a donation for their wisdom] {requires: gold >= 15}
  ~ gain_radical("白")
  "白" (bái) — white, pure, clear
* [Politely decline and continue]
  ~ nothing
  "别" (bié) — don't, farewell
