=== EVENT_PIRATE_AMBUSH ===
# id: 7
# title: Pirate Ambush
# chinese_title: 海盗伏击
# category: PirateEncounter
# mode: starmap

Proximity alarms blare as two pirate fighters decloak off your bow. Their leader broadcasts: 'Pay up or we paint the void with your hull.'

* [Fight them off]
  ~ combat_reward(2, 25)
  战斗 (zhàndòu) — fight
* [Pay the tribute] {requires: credits >= 20}
  ~ lose_credits(20)
  付钱 (fùqián) — pay
* [Bluff — claim you are a military vessel] {requires: class == 1}
  ~ nothing
  欺骗 (qīpiàn) — bluff
* [Attempt to flee at full burn] {requires: fuel >= 4}
  ~ lose_fuel(4)
  逃跑 (táopǎo) — flee
