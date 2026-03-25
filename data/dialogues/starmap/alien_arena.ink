=== EVENT_ALIEN_ARENA ===
# id: 69
# title: Alien Arena
# chinese_title: 外星竞技场
# category: AlienContact
# mode: starmap

An alien species runs a gladiatorial arena aboard their station. They challenge all visitors to prove their worth in combat. Victory brings glory and reward.

* [[Combat level 3, +35 credits] Enter the arena!]
  ~ combat_reward(3, 35)
  斗 (dòu) — fight
* [[Combat level 5, +60 credits] Challenge the champion]
  ~ combat_reward(5, 60)
  冠军 (guànjūn) — champion
* [[-20 credits] Bet on another fighter] {requires: credits >= 20}
  ~ gain_credits(30)
  赌 (dǔ) — gamble
* [[+15 credits] Sell refreshments to the crowd]
  ~ gain_credits(15)
  卖 (mài) — sell
* [[Gain radical 角] Study the alien fighting styles]
  ~ gain_radical("角")
  角 (jiǎo) — horn, angle
