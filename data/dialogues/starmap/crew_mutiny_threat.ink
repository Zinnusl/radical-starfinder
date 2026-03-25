=== EVENT_CREW_MUTINY_THREAT ===
# id: 76
# title: Crew Mutiny Threat
# chinese_title: 船员哗变威胁
# category: CrewEvent
# mode: starmap

A delegation of crew members blocks the bridge entrance. Their leader speaks with barely controlled fury: 'Captain, we've had enough. Conditions are unbearable. Change course or we take the ship.'

* [[-20 credits] Promise better rations and shore leave] {requires: credits >= 20}
  ~ heal_crew(20)
  承诺 (chéngnuò) — promise
* [[Lose crew member] Lock down the ringleader in the brig]
  ~ lose_crew_member
  镇压 (zhènyā) — suppress
* [[-5 fuel] Divert to the nearest station for shore leave] {requires: fuel >= 5}
  ~ fuel_and_credits(-5, 0)
  休假 (xiūjià) — leave, vacation
* [[-10 hull] Crew sabotages systems before you regain control]
  ~ lose_hull(10)
  破坏 (pòhuài) — sabotage
