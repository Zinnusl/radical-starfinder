=== EVENT_CHRONO_MERCHANT ===
# id: 52
# title: Chrono Merchant
# chinese_title: 时间商人
# category: Trading
# mode: starmap

A merchant from a time-dilated sector offers exotic wares. Their goods shimmer with temporal energy. Prices are steep but the merchandise is unlike anything you've seen.

* [[-50 credits, +30 hull] Buy temporal hull plating] {requires: credits >= 50}
  ~ gain_hull(30)
  盾 (dùn) — shield
* [[-35 credits, +20 fuel] Purchase condensed time-fuel] {requires: credits >= 35}
  ~ gain_fuel(20)
  燃料 (ránliào) — fuel
* [[Gain radical 门] Trade knowledge of thresholds]
  ~ gain_radical("门")
  门 (mén) — door, gate
* [[-20 credits, Gain item] Buy a Chrono Stabilizer] {requires: credits >= 20}
  ~ gain_item("Chrono Stabilizer")
  买 (mǎi) — buy
* [[+15 credits] Sell scrap from your cargo hold]
  ~ gain_credits(15)
  卖 (mài) — sell
