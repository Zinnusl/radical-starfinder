=== EVENT_PIRATE_DEFECTOR ===
# id: 10
# title: Pirate Defector
# chinese_title: 海盗叛逃者
# category: PirateEncounter
# mode: starmap

A small shuttle approaches with weapons powered down. The pilot claims to be a pirate deserter seeking asylum.

* [Welcome them aboard]
  ~ gain_crew_member
  加入 (jiārù) — join
* [Demand intel on pirate routes as payment]
  ~ gain_item("Pirate Cipher Key")
  情报 (qíngbào) — intel
* [It could be a trap — drive them away]
  ~ nothing
  陷阱 (xiànjǐng) — trap
