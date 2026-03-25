=== EVENT_DAMAGED_FREIGHTER ===
# id: 2
# title: Damaged Freighter
# chinese_title: 受损货船
# category: DistressSignal
# mode: starmap

A massive freighter lists to starboard, venting atmosphere from multiple breaches. The captain hails you on emergency frequencies.

* [Send your engineer to help repair] {requires: crew_role == 1}
  ~ gain_credits(25)
  修理 (xiūlǐ) — repair
* [Board and loot the cargo hold]
  ~ gain_scrap(15)
  货物 (huòwù) — cargo
* [Offer fuel in exchange for credits] {requires: fuel >= 5}
  ~ fuel_and_credits(-5, 30)
  交换 (jiāohuàn) — exchange
