=== EVENT_BLACK_MARKET ===
# id: 14
# title: Black Market
# chinese_title: 黑市
# category: Trading
# mode: starmap

Hidden among a cluster of derelict hulls is a thriving black market station. The goods are questionable, but the prices are tempting.

* [Buy military-grade weapons] {requires: credits >= 25}
  ~ gain_item("Ion Disruptor")
  武器 (wǔqì) — weapon
* [Sell your scrap for top credit]
  ~ gain_credits(20)
  卖 (mài) — sell
* [Gamble in the fight pit]
  ~ combat_reward(1, 35)
  赌博 (dǔbó) — gamble
* [Leave — this place feels wrong]
  ~ nothing
  离开 (líkāi) — leave
