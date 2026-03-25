=== EVENT_STRANDED_MINERS ===
# id: 4
# title: Stranded Miners
# chinese_title: 被困矿工
# category: DistressSignal
# mode: starmap

A mining crew is trapped on an asteroid after their shuttle broke down. They wave frantically through the viewport.

* [Evacuate the miners aboard your ship]
  ~ gain_crew_member
  矿工 (kuànggōng) — miner
* [Tow their shuttle to the nearest station (costs fuel)] {requires: fuel >= 6}
  ~ fuel_and_credits(-6, 35)
  拖 (tuō) — tow
* [Take their ore stockpile as payment for rescue]
  ~ gain_scrap(20)
  矿石 (kuàngshí) — ore
