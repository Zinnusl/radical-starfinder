=== EVENT_MEDICAL_FRIGATE ===
# id: 6
# title: Medical Frigate
# chinese_title: 医疗护卫舰
# category: DistressSignal
# mode: starmap

A medical frigate hails you with a request: they need an escort through pirate territory. In return, they can treat your crew.

* [Escort them (risk pirate attack)]
  ~ heal_crew(30)
  护送 (hùsòng) — escort
* [Ask for medical supplies instead of escorting]
  ~ heal_crew(15)
  药品 (yàopǐn) — medical supplies
* [Decline — you have your own problems]
  ~ nothing
  拒绝 (jùjué) — decline
