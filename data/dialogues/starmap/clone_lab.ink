=== EVENT_CLONE_LAB ===
# id: 65
# title: Clone Lab
# chinese_title: 克隆实验室
# category: CrewEvent
# mode: starmap

A derelict station contains functional cloning vats. Your chief medical officer suggests using them, but the technology raises ethical concerns among the crew.

* [[Gain crew member, -10 hull] Activate the cloning sequence]
  ~ gain_crew_member
  复制 (fùzhì) — copy
* [[+20 credits] Sell the cloning data]
  ~ gain_credits(20)
  数据 (shùjù) — data
* [[Heal 5 HP] Use the medical equipment for healing]
  ~ heal_crew(5)
  医 (yī) — medical
* [[Gain radical 身] Study the bio-patterns]
  ~ gain_radical("身")
  身 (shēn) — body
* [[-5 hull] Destroy the lab to prevent misuse]
  ~ lose_hull(5)
  毁 (huǐ) — destroy
