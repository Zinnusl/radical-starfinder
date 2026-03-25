=== FUEL_MERCHANT ===
# id: 22
# title: The Fuel Merchant
# chinese_title: 燃料商
# category: Trader
# mode: dungeon

A compact ship has docked at the station's fuel depot, its hull emblazoned with the logo of an independent fuel cooperative. The captain offers refined plasma at competitive rates, along with additives that boost engine efficiency.

* [Purchase premium fuel additives] {requires: gold >= 20}
  ~ gain_radical("火")
  "火" (huǒ) — fire, flame
* [Buy standard fuel rations] {requires: gold >= 10}
  ~ gain_xp(10)
  "油" (yóu) — oil, fuel
* [Negotiate a bulk discount]
  ~ gain_gold(10)
  "省" (shěng) — to save, to economize
* [Ask about fuel depot rumors]
  ~ gain_xp(15)
  "闻" (wén) — to hear, news
