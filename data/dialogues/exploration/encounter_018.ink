=== ARMS_DEALER ===
# id: 17
# title: The Arms Dealer
# chinese_title: 军火商
# category: Trader
# mode: dungeon

A heavyset man with burn scars across his forearms leans against a reinforced crate. "Looking for something with a bit more punch?" he growls, sliding open the lid to reveal an arsenal of modified weapons. Each piece bears custom engravings and aftermarket modifications.

* [Purchase a weapon upgrade] {requires: gold >= 30}
  ~ gain_equipment
  "武" (wǔ) — martial, weapon, military
* [Ask about his custom modifications]
  ~ gain_xp(15)
  "改" (gǎi) — to change, to modify
* [Trade intel for a discount]
  ~ gain_radical("刀")
  "刀" (dāo) — knife, blade
* [Decline politely]
  ~ nothing
  "谢" (xiè) — to thank, to decline
