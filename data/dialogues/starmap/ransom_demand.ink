=== EVENT_RANSOM_DEMAND ===
# id: 12
# title: Ransom Demand
# chinese_title: 勒索要求
# category: PirateEncounter
# mode: starmap

Pirates have captured a passenger liner and demand ransom. They broadcast the terrified passengers on an open channel.

* [Pay the ransom to free the passengers] {requires: credits >= 15}
  ~ fuel_and_credits(-2, -15)
  赎金 (shújīn) — ransom
* [Attack the pirates to free the hostages]
  ~ combat_reward(3, 20)
  解救 (jiějiù) — rescue
* [Report the situation and continue on your way]
  ~ nothing
  报告 (bàogào) — report
