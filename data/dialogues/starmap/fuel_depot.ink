=== EVENT_FUEL_DEPOT ===
# id: 15
# title: Fuel Depot
# chinese_title: 燃料补给站
# category: Trading
# mode: starmap

An automated fuel depot orbits a gas giant. Its prices are fair and the pumps are fast.

* [Buy a full tank] {requires: credits >= 12}
  ~ fuel_and_credits(10, -12)
  加满 (jiāmǎn) — fill up
* [Buy a partial refuel] {requires: credits >= 6}
  ~ fuel_and_credits(5, -6)
  一些 (yīxiē) — some
* [Hack the pumps for free fuel] {requires: crew_role == 3}
  ~ gain_fuel(6)
  黑客 (hēikè) — hacker
