=== EVENT_CREW_CELEBRATION ===
# id: 34
# title: Crew Celebration
# chinese_title: 船员庆祝
# category: CrewEvent
# mode: starmap

The crew wants to celebrate a milestone — 100 jumps together. They request shore leave at the next station.

* [Grant shore leave (costs credits)] {requires: credits >= 10}
  ~ heal_crew(20)
  庆祝 (qìngzhù) — celebrate
* [Throw a party on the ship]
  ~ heal_crew(10)
  派对 (pàiduì) — party
* [No time for celebrations — push on]
  ~ damage_crew(3)
  没时间 (méi shíjiān) — no time
