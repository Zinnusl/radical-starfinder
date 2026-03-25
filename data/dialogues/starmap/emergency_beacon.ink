=== EVENT_EMERGENCY_BEACON ===
# id: 73
# title: Emergency Beacon
# chinese_title: 紧急信标
# category: DistressSignal
# mode: starmap

An automated distress beacon leads you to a damaged military corvette. The surviving officer offers classified intel and supplies in exchange for escort to the nearest station.

* [[-10 fuel, +40 credits] Escort them for full payment] {requires: fuel >= 10}
  ~ fuel_and_credits(-10, 40)
  护送 (hùsòng) — escort
* [[+20 hull] Accept hull repair kits as partial payment]
  ~ gain_hull(20)
  修理 (xiūlǐ) — repair
* [[Gain crew member] The officer joins your crew]
  ~ gain_crew_member
  军人 (jūnrén) — soldier
* [[+25 credits, -5 hull] Salvage their damaged ship for parts]
  ~ gain_credits(25)
  拆 (chāi) — dismantle
* [[Combat level 2, +30 credits] Betray them and take everything]
  ~ combat_reward(2, 30)
  背叛 (bèipàn) — betray
