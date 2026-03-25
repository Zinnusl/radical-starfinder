=== EVENT_RADIATION_BELT ===
# id: 42
# title: Radiation Belt
# chinese_title: 辐射带
# category: HazardEvent
# mode: starmap

A dense radiation belt blocks your planned route. Your medical officer warns of crew exposure risks.

* [Go through with radiation shielding]
  ~ damage_crew(8)
  辐射 (fúshè) — radiation
* [Detour around the belt] {requires: fuel >= 6}
  ~ lose_fuel(6)
  绕路 (ràolù) — detour
* [Wait for a gap in the radiation]
  ~ lose_fuel(2)
  耐心 (nàixīn) — patience
