=== EVENT_TRADE_STATION ===
# id: 17
# title: Trade Station
# chinese_title: 贸易站
# category: Trading
# mode: starmap

A bustling orbital station invites all ships to dock. Merchants from a dozen species hawk their wares in a cacophony of languages.

* [Trade scrap for fuel]
  ~ gain_fuel(5)
  换 (huàn) — exchange
* [Visit the shipyard for repairs] {requires: credits >= 20}
  ~ repair_ship(15)
  船坞 (chuánwù) — shipyard
* [Recruit a crew member from the bar] {requires: credits >= 10}
  ~ gain_crew_member
  招募 (zhāomù) — recruit
