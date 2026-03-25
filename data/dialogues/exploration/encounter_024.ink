=== TECH_SALVAGER ===
# id: 23
# title: The Tech Salvager
# chinese_title: 科技回收者
# category: Trader
# mode: dungeon

A grease-stained mechanic has spread her haul across a workbench: circuit boards, power cells, and a device that looks suspiciously military. She picks up a wrench and gestures at her wares. "Everything works. Mostly. Test before you buy, I always say."

* [Buy the military-grade device] {requires: gold >= 35}
  ~ gain_equipment
  "器" (qì) — device, instrument
* [Purchase power cells] {requires: gold >= 15}
  ~ gain_item("emp_grenade")
  "电" (diàn) — electricity
* [Offer to help repair something in exchange] {requires: class == 1}
  ~ gain_radical("手")
  "手" (shǒu) — hand
* [Browse the circuit boards]
  ~ gain_xp(10)
  "板" (bǎn) — board, panel
