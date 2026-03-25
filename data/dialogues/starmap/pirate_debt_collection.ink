=== EVENT_PIRATE_DEBT_COLLECTION ===
# id: 74
# title: Pirate Debt Collection
# chinese_title: 海盗追债
# category: PirateEncounter
# mode: starmap

A heavily armed pirate enforcer drops out of hyperspace and hails you. 'You've made enemies among the fleet. Time to settle accounts — with credits or with blood.'

* [[-30 credits] Pay what they demand and hope they leave] {requires: credits >= 30}
  ~ lose_credits(30)
  还债 (huánzhài) — repay a debt
* [[Combat level 3, +35 credits] Fight the enforcer]
  ~ combat_reward(3, 35)
  反抗 (fǎnkàng) — resist
* [[-8 fuel] Dump fuel to create a flashpoint and flee] {requires: fuel >= 8}
  ~ lose_fuel(8)
  逃 (táo) — escape
* [[-10 hull] Take a hit while escaping through an asteroid field]
  ~ lose_hull(10)
  损伤 (sǔnshāng) — damage
