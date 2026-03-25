=== WATER_FLOW_PUZZLE ===
# id: 56
# title: The Water Flow Puzzle
# chinese_title: 水流谜题
# category: Puzzle
# mode: dungeon

An engineering section has been flooded knee-deep. A series of valves and channels must be directed to drain the water and reveal the floor hatch beneath. Markings on each valve correspond to water-related characters.

* [Read the markings and turn the correct valves]
  ~ gain_radical("水")
  "流" (liú) — to flow, current, stream
* [Follow the pipe system to trace the flow]
  ~ gain_xp(20)
  "管" (guǎn) — pipe, tube, to manage
* [Wade through to the hatch directly]
  ~ damage(3)
  "深" (shēn) — deep
* [Use a pump to force-drain the area] {requires: class == 1}
  ~ gain_xp(25)
  "排" (pái) — to drain, to arrange
