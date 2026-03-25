=== DRUNKEN_PILOT ===
# id: 51
# title: The Drunken Pilot
# chinese_title: 醉酒飞行员
# category: Crew
# mode: dungeon

A pilot sits in a grounded shuttle, surrounded by empty nutrient paste tubes and a flask of something potent. She claims she knows a shortcut through the debris field outside, but her slurred speech doesn't inspire confidence. Her navigation data, however, looks surprisingly solid.

* [Copy her navigation data]
  ~ gain_xp(25)
  "航" (háng) — navigation, to sail
* [Help her sober up with a stim pack]
  ~ gain_radical("口")
  "酒" (jiǔ) — alcohol, wine. The radical "口" relates to mouth/drinking.
* [Buy the nav data outright] {requires: gold >= 20}
  ~ gain_gold(40)
  "钱" (qián) — money
* [Ignore the drunkard]
  ~ nothing
  "醉" (zuì) — drunk, intoxicated
