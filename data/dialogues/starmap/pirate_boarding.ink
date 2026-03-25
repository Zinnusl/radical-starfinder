=== EVENT_PIRATE_BOARDING ===
# id: 9
# title: Boarding Party
# chinese_title: 登船突击
# category: PirateEncounter
# mode: starmap

Pirates lock onto your airlock with a boarding tube. You can hear the cutting torch on the other side of the hull.

* [Repel the boarders in close combat]
  ~ combat_reward(2, 15)
  抵抗 (dǐkàng) — resist
* [Vent the compartment into space]
  ~ lose_hull(5)
  真空 (zhēnkōng) — vacuum
* [Surrender your cargo] {requires: credits >= 30}
  ~ lose_credits(30)
  投降 (tóuxiáng) — surrender
