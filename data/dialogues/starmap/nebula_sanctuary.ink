=== EVENT_NEBULA_SANCTUARY ===
# id: 59
# title: Nebula Sanctuary
# chinese_title: 星云庇护所
# category: AlienContact
# mode: starmap

Hidden within a dense nebula, an alien monastery floats serenely. Monks of an ancient order offer wisdom, healing, and trade to peaceful visitors.

* [[Heal 8 HP] Receive their healing blessing]
  ~ heal_crew(8)
  治 (zhì) — heal, cure
* [[Gain radical 大] Learn their philosophy of expansion]
  ~ gain_radical("大")
  大 (dà) — big, great
* [[-20 credits, +25 fuel] Trade for purified nebula fuel] {requires: credits >= 20}
  ~ gain_fuel(25)
  净 (jìng) — pure
* [[Gain crew member] A monk wishes to join your journey]
  ~ gain_crew_member
  僧 (sēng) — monk
* [[+15 credits] Trade your stories for their gifts]
  ~ gain_credits(15)
  故事 (gùshì) — story
