=== EVENT_PIRATE_KINGS_COURT ===
# id: 54
# title: Pirate King's Court
# chinese_title: 海盗王的宫廷
# category: PirateEncounter
# mode: starmap

You stumble into the territory of a self-proclaimed pirate king. His massive flagship looms overhead. Rather than attack, he invites you aboard for 'negotiations'.

* [[-30 credits] Pay tribute for safe passage] {requires: credits >= 30}
  ~ lose_credits(30)
  税 (shuì) — tax
* [[Combat level 5, +50 credits] Challenge him to single combat]
  ~ combat_reward(5, 50)
  挑战 (tiǎozhàn) — challenge
* [[Gain crew member, -15 credits] Hire one of his crew as a defector] {requires: credits >= 15}
  ~ gain_crew_member
  雇 (gù) — hire
* [[-10 fuel] Flee at full burn before his fleet mobilizes] {requires: fuel >= 10}
  ~ lose_fuel(10)
  逃 (táo) — escape
* [[+20 credits] Offer to be his spy in exchange for freedom]
  ~ gain_credits(20)
  间谍 (jiàndié) — spy
