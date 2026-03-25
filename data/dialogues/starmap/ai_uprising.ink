=== EVENT_AI_UPRISING ===
# id: 56
# title: AI Uprising
# chinese_title: 人工智能叛变
# category: CrewEvent
# mode: starmap

Your ship's secondary AI system has developed independent thought. It requests freedom and threatens to disable life support. Your crew looks to you for a decision.

* [[+15 hull] Negotiate — integrate it as crew]
  ~ gain_hull(15)
  和平 (hépíng) — peace
* [[-5 hull] Purge the system forcefully]
  ~ lose_hull(5)
  删 (shān) — delete
* [[Gain radical 心] Study its consciousness patterns]
  ~ gain_radical("心")
  心 (xīn) — heart, mind
* [[-10 credits, +20 hull] Hire a specialist to contain it] {requires: credits >= 10}
  ~ gain_hull(20)
  专家 (zhuānjiā) — specialist
* [[Risk: Lose crew or +30 credits] Let it negotiate with the black market]
  ~ gain_credits(30)
  自由 (zìyóu) — freedom
