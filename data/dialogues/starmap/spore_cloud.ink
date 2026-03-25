=== EVENT_SPORE_CLOUD ===
# id: 62
# title: Spore Cloud
# chinese_title: 孢子云
# category: HazardEvent
# mode: starmap

A massive cloud of alien spores engulfs your ship. They begin eating through the hull but seem to have medicinal properties if properly processed.

* [[-10 hull, Heal 5 HP] Process spores into medicine]
  ~ heal_crew(5)
  药 (yào) — medicine
* [[-5 hull] Activate hull scrubbers to purge them]
  ~ lose_hull(5)
  清 (qīng) — clean
* [[+20 credits, -15 hull] Collect spores for sale to researchers]
  ~ gain_credits(20)
  卖 (mài) — sell
* [[-15 fuel] Full burn to escape the cloud] {requires: fuel >= 15}
  ~ lose_fuel(15)
  逃 (táo) — escape
* [[Gain radical 虫] Study the spore lifecycle]
  ~ gain_radical("虫")
  虫 (chóng) — insect, bug
