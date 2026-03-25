=== EVENT_GRAVITY_WELL ===
# id: 28
# title: Gravity Well
# chinese_title: 引力阱
# category: AnomalyEncounter
# mode: starmap

You stumble into an invisible gravity well. The ship groans as unseen forces pull you toward a collapsed star.

* [Full burn to escape (costs fuel)] {requires: fuel >= 6}
  ~ lose_fuel(6)
  逃脱 (táotuō) — escape
* [Use a gravity slingshot maneuver] {requires: crew_role == 0}
  ~ gain_fuel(3)
  引力 (yǐnlì) — gravity
* [Brace for impact and ride it out]
  ~ lose_hull(10)
  坚持 (jiānchí) — endure
