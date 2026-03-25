=== EVENT_DISTRESS_BEACON ===
# id: 0
# title: Distress Beacon
# chinese_title: 求救信号
# category: DistressSignal
# mode: starmap

Your sensors detect a faint distress beacon from a drifting vessel. The hull is cracked and life signs are weak but present.

* [Dock and render aid (costs fuel)] {requires: fuel >= 3}
  ~ fuel_and_credits(-3, 15)
  帮助 (bāngzhù) — help
* [Salvage what you can from the wreckage]
  ~ gain_scrap(10)
  残骸 (cánhái) — wreckage
* [Ignore the signal and move on]
  ~ nothing
  忽略 (hūlüè) — ignore
