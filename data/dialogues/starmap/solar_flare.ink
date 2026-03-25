=== EVENT_SOLAR_FLARE ===
# id: 40
# title: Solar Flare
# chinese_title: 太阳耀斑
# category: HazardEvent
# mode: starmap

Warning: the local star erupts in a massive solar flare. Radiation levels spike and your shields strain under the bombardment.

* [Angle shields and ride it out]
  ~ shield_damage(8)
  盾牌 (dùnpái) — shield
* [Emergency FTL jump away] {requires: fuel >= 5}
  ~ lose_fuel(5)
  紧急 (jǐnjí) — emergency
* [Hide in the shadow of a nearby planet]
  ~ lose_fuel(2)
  影子 (yǐngzi) — shadow
