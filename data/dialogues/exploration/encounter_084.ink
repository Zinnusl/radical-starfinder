=== SHATTERED_BRIDGE ===
# id: 83
# title: The Shattered Bridge
# chinese_title: 破碎舰桥
# category: Wreckage
# mode: dungeon

The command bridge of a destroyed warship lies open to the stars, its viewport shattered. Control consoles spark intermittently, and the captain's chair is slashed in half. Despite the destruction, some systems still flicker with power. Emergency data cores may have survived.

* [Salvage the emergency data cores]
  ~ gain_gold(30)
  "碎" (suì) — shattered, broken, pieces
* [Search the captain's personal effects]
  ~ gain_radical("王")
  "船" (chuán) — ship, vessel. The radical "王" relates to command authority.
* [Patch into the remaining power systems] {requires: class == 1}
  ~ gain_equipment
  "接" (jiē) — to connect, to patch
* [Scavenge components from the consoles]
  ~ gain_item("scanner_pulse")
  "拆" (chāi) — to dismantle, to take apart
