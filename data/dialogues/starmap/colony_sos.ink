=== EVENT_COLONY_SOS ===
# id: 3
# title: Colony Distress Call
# chinese_title: 殖民地求救
# category: DistressSignal
# mode: starmap

A frontier colony broadcasts an urgent plea: plague has struck and medical supplies are exhausted. They offer everything they have.

* [Deliver emergency supplies] {requires: fuel >= 4}
  ~ fuel_and_credits(-4, 20)
  医药 (yīyào) — medicine
* [Trade supplies for their rare artifacts]
  ~ gain_radical("疒")
  古物 (gǔwù) — artifact
* [Log the coordinates and move on]
  ~ nothing
  记录 (jìlù) — record
