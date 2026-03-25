=== EVENT_MERCENARY_OUTPOST ===
# id: 63
# title: Mercenary Outpost
# chinese_title: 雇佣兵前哨
# category: PirateEncounter
# mode: starmap

A well-armed mercenary company operates from this station. They offer their services — or might just take what they want if you look weak enough.

* [[-25 credits, Gain crew member] Hire a veteran fighter] {requires: credits >= 25}
  ~ gain_crew_member
  兵 (bīng) — soldier
* [[Combat level 3, +35 credits] Accept a contract job]
  ~ combat_reward(3, 35)
  合同 (hétóng) — contract
* [[-15 credits, Gain item] Buy weapon upgrades] {requires: credits >= 15}
  ~ gain_item("Weapon Mod")
  武器 (wǔqì) — weapon
* [[+20 credits] Sell surplus equipment]
  ~ gain_credits(20)
  卖 (mài) — sell
* [[Nothing] Leave before they get ideas]
  ~ nothing
  离开 (líkāi) — leave
