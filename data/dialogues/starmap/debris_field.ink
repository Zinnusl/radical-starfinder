=== EVENT_DEBRIS_FIELD ===
# id: 39
# title: Debris Field
# chinese_title: 碎片区域
# category: HazardEvent
# mode: starmap

You enter a field of shattered ships and tumbling rock. Something catastrophic happened here. Navigation is treacherous.

* [Carefully navigate through]
  ~ lose_fuel(3)
  导航 (dǎoháng) — navigate
* [Salvage as you go (risk hull damage)]
  ~ hull_and_fuel(-5, 0)
  打捞 (dǎlāo) — salvage
* [Power through at full speed]
  ~ lose_hull(12)
  全速 (quánsù) — full speed
