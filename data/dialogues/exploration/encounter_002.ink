=== HIDDEN_COMPARTMENT ===
# id: 1
# title: The Hidden Compartment
# chinese_title: 密室
# category: Discovery
# mode: dungeon

While inspecting a decommissioned bulkhead, your fingers find a seam that shouldn't be there. A concealed panel slides open, revealing a cramped space filled with pre-war supplies. Dust motes swirl in the stale air, undisturbed for decades.

* [Search through the supplies]
  ~ gain_gold(25)
  "找" (zhǎo) — to search, to look for
* [Check for traps before entering]
  ~ gain_item("scanner_pulse")
  "查" (chá) — to check, to examine
* [Take the medical supplies]
  ~ gain_item("med_hypo")
  "药" (yào) — medicine, drug
* [Seal it back up — someone hid this for a reason]
  ~ gain_xp(10)
  "封" (fēng) — to seal, to close
