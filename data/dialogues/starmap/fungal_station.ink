=== EVENT_FUNGAL_STATION ===
# id: 53
# title: Fungal Station
# chinese_title: 真菌空间站
# category: HazardEvent
# mode: starmap

An abandoned station overrun with bioluminescent fungi. Spores drift through breached corridors. The growth appears to have consumed the original crew, but valuable equipment may remain.

* [[Risk: -15 hull or +25 credits] Send a team to salvage]
  ~ fuel_and_credits(0, 20)
  搜索 (sōusuǒ) — search
* [[+10 hull] Harvest fungal compounds for bio-adhesive]
  ~ gain_hull(10)
  采 (cǎi) — harvest
* [[Gain radical 木] Study the growth patterns]
  ~ gain_radical("木")
  木 (mù) — wood, tree
* [[-10 fuel] Burn it out with your engines and loot safely] {requires: fuel >= 10}
  ~ gain_credits(25)
  火 (huǒ) — fire
* [[Nothing] Mark it on charts and move on]
  ~ nothing
  记 (jì) — record
