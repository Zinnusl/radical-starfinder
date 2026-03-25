=== EVENT_SOLAR_FORGE ===
# id: 70
# title: Solar Forge
# chinese_title: 太阳熔炉
# category: AncientRuins
# mode: starmap

An ancient structure orbiting close to a star harnesses its energy to forge exotic metals. The heat is extreme, but the materials inside are priceless.

* [[-15 hull, +45 credits] Mine the forged metals]
  ~ gain_credits(45)
  铁 (tiě) — iron
* [[+25 hull] Use the forge to reinforce your hull]
  ~ gain_hull(25)
  强 (qiáng) — strengthen
* [[Gain radical 金] Study the metallurgical inscriptions]
  ~ gain_radical("金")
  金 (jīn) — gold, metal
* [[+20 fuel] Convert stellar energy to fuel]
  ~ gain_fuel(20)
  太阳 (tàiyáng) — sun
* [[-10 fuel] Retreat to safe distance and scan remotely] {requires: fuel >= 10}
  ~ lose_fuel(10)
  安全 (ānquán) — safe
