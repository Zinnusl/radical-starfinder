=== EVENT_GHOST_FLEET ===
# id: 71
# title: Ghost Fleet
# chinese_title: 幽灵舰队
# category: HazardEvent
# mode: starmap

Dozens of derelict warships drift in formation, their weapons cold but their reactors still humming. Something wiped them out simultaneously. Warning beacons have long since failed.

* [[+20 fuel, -5 hull] Siphon reactor fuel from the nearest ship]
  ~ hull_and_fuel(-5, 20)
  吸 (xī) — absorb
* [[+30 credits, -10 hull] Loot the flagship's vault]
  ~ gain_credits(30)
  宝 (bǎo) — treasure
* [[Gain radical 目] Investigate what killed them]
  ~ gain_radical("目")
  目 (mù) — eye
* [[+15 hull] Salvage intact hull sections]
  ~ gain_hull(15)
  修 (xiū) — repair
* [[Nothing] Too dangerous — leave immediately]
  ~ nothing
  危 (wēi) — danger
