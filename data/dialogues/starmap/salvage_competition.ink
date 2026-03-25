=== EVENT_SALVAGE_COMPETITION ===
# id: 67
# title: Salvage Competition
# chinese_title: 打捞竞赛
# category: Trading
# mode: starmap

Multiple salvage teams converge on a massive derelict capital ship. The station master declares a salvage competition — first come, first served, with prizes for the most valuable haul.

* [[Risk: -10 hull or +40 credits] Race to the engineering section]
  ~ gain_credits(35)
  快 (kuài) — fast
* [[-10 fuel, +30 credits] Use your engines to reach cargo first] {requires: fuel >= 10}
  ~ fuel_and_credits(-10, 30)
  先 (xiān) — first
* [[Combat level 2, +35 credits] Fight other teams for the best loot]
  ~ combat_reward(2, 35)
  争 (zhēng) — compete
* [[+15 credits] Play it safe, scavenge the outer hull]
  ~ gain_credits(15)
  安全 (ānquán) — safe
* [[-20 credits, +25 hull] Buy the rights to the bridge section] {requires: credits >= 20}
  ~ gain_hull(25)
  桥 (qiáo) — bridge
