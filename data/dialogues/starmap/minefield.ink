=== EVENT_MINEFIELD ===
# id: 41
# title: Minefield
# chinese_title: 雷区
# category: HazardEvent
# mode: starmap

Proximity sensors scream — you have wandered into a dormant minefield left over from a forgotten war.

* [Carefully reverse course]
  ~ lose_fuel(4)
  后退 (hòutuì) — reverse
* [Use your engineer to disarm a path] {requires: crew_role == 1}
  ~ gain_scrap(10)
  拆除 (chāichú) — disarm
* [Push through and hope for the best]
  ~ lose_hull(15)
  运气 (yùnqi) — luck
