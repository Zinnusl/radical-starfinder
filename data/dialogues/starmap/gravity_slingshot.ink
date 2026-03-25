=== EVENT_GRAVITY_SLINGSHOT ===
# id: 60
# title: Gravity Slingshot
# chinese_title: 重力弹弓
# category: AnomalyEncounter
# mode: starmap

Twin black holes create a narrow corridor of stable space between them. Threading the gap could fling you far ahead — or tear your ship apart.

* [[+25 fuel, -10 hull] Thread the gap at full speed]
  ~ hull_and_fuel(-10, 25)
  快 (kuài) — fast
* [[-5 fuel] Take the long way around] {requires: fuel >= 5}
  ~ lose_fuel(5)
  慢 (màn) — slow
* [[Risk: -20 hull or +20 fuel, +20 credits] Ride the gravity wave]
  ~ fuel_and_credits(15, 15)
  浪 (làng) — wave
* [[+15 credits] Deploy probes to study the phenomenon]
  ~ gain_credits(15)
  研究 (yánjiū) — research
* [[Gain radical 力] Meditate on the forces at play]
  ~ gain_radical("力")
  力 (lì) — power, force
