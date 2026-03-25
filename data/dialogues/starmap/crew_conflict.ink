=== EVENT_CREW_CONFLICT ===
# id: 31
# title: Crew Conflict
# chinese_title: 船员冲突
# category: CrewEvent
# mode: starmap

Two crew members have come to blows over rations. The situation threatens to split the crew into factions.

* [Mediate the dispute personally] {requires: class == 4}
  ~ heal_crew(5)
  调解 (tiáojiě) — mediate
* [Let them sort it out themselves]
  ~ damage_crew(5)
  自己 (zìjǐ) — themselves
* [Put both in the brig until they cool down]
  ~ nothing
  禁闭 (jìnbì) — confine
