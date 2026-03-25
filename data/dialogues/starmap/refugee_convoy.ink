=== EVENT_REFUGEE_CONVOY ===
# id: 51
# title: Refugee Convoy
# chinese_title: 难民车队
# category: DistressSignal
# mode: starmap

A convoy of civilian ships flees a destroyed colony. Their leader hails you desperately — they need fuel, repairs, and protection from pursuing raiders.

* [[-15 fuel, +40 credits] Share fuel supplies for payment] {requires: fuel >= 15}
  ~ fuel_and_credits(-15, 40)
  帮 (bāng) — help
* [[+20 hull, -10 credits] Trade repair parts] {requires: credits >= 10}
  ~ gain_hull(20)
  修 (xiū) — repair
* [[Gain crew member, -5 fuel] Take refugees aboard]
  ~ gain_crew_member
  人 (rén) — person
* [[Combat level 3, +30 credits] Fight off the pursuing raiders]
  ~ combat_reward(3, 30)
  保护 (bǎohù) — protect
* [[-5 hull] Ignore them and push through the debris]
  ~ lose_hull(5)
  忽略 (hūlüè) — ignore
