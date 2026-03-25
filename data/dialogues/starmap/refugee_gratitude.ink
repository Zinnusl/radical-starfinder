=== EVENT_REFUGEE_GRATITUDE ===
# id: 75
# title: Refugee Gratitude
# chinese_title: 难民感恩
# category: CrewEvent
# mode: starmap

A small convoy intercepts you with a friendly hail. 'Captain! We are the refugees you helped before. We've rebuilt and want to repay your kindness.'

* [[+25 credits, +5 fuel] Accept their generous gift package]
  ~ fuel_and_credits(5, 25)
  感谢 (gǎnxiè) — gratitude
* [[Gain crew member] Welcome a skilled refugee aboard]
  ~ gain_crew_member
  欢迎 (huānyíng) — welcome
* [[+15 hull] Accept hull repair materials they salvaged]
  ~ repair_ship(15)
  材料 (cáiliào) — materials
* [[Gain radical 恩] Decline — their freedom is reward enough]
  ~ gain_radical("恩")
  恩 (ēn) — grace, kindness
