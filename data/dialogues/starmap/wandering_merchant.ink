=== EVENT_WANDERING_MERCHANT ===
# id: 13
# title: Wandering Merchant
# chinese_title: 流浪商人
# category: Trading
# mode: starmap

A battered cargo hauler pulls alongside. Its eccentric captain opens a channel: 'Best deals in the sector! Everything must go!'

* [Buy fuel reserves] {requires: credits >= 10}
  ~ fuel_and_credits(8, -10)
  燃料 (ránliào) — fuel
* [Buy hull repair nanites] {requires: credits >= 15}
  ~ hull_and_fuel(10, 0)
  修复 (xiūfù) — repair
* [Browse the curiosities]
  ~ gain_radical("贝")
  好奇 (hàoqí) — curious
* [Decline and move on]
  ~ nothing
  不用 (bùyòng) — no thanks
