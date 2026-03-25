=== EVENT_ABANDONED_SHIPYARD ===
# id: 61
# title: Abandoned Shipyard
# chinese_title: 废弃船坞
# category: Discovery
# mode: starmap

A massive orbital shipyard drifts silently, its construction bays still holding half-built vessels. Automated defense turrets flicker intermittently.

* [[+25 hull] Salvage hull plating from unfinished ships]
  ~ gain_hull(25)
  修 (xiū) — repair
* [[+20 fuel] Drain fuel reserves from docked ships]
  ~ gain_fuel(20)
  油 (yóu) — fuel, oil
* [[-15 credits, Gain item] Buy a blueprint from the data core] {requires: credits >= 15}
  ~ gain_item("Ship Blueprint")
  图 (tú) — diagram
* [[Combat level 3, +40 credits] Fight through turrets to the vault]
  ~ combat_reward(3, 40)
  打 (dǎ) — fight
* [[+15 credits] Scavenge loose parts from the exterior]
  ~ gain_credits(15)
  捡 (jiǎn) — pick up
