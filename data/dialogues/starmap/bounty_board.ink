=== EVENT_BOUNTY_BOARD ===
# id: 58
# title: Bounty Board
# chinese_title: 悬赏布告
# category: Trading
# mode: starmap

A relay station broadcasts active bounties. Several targets are in nearby sectors. Taking a bounty means combat, but the rewards are substantial.

* [[Combat level 2, +25 credits] Hunt the smuggler]
  ~ combat_reward(2, 25)
  猎 (liè) — hunt
* [[Combat level 4, +45 credits] Track the pirate captain]
  ~ combat_reward(4, 45)
  海盗 (hǎidào) — pirate
* [[Combat level 3, +35 credits] Capture the rogue AI ship]
  ~ combat_reward(3, 35)
  捕 (bǔ) — capture
* [[-15 credits] Buy intel on bounty locations] {requires: credits >= 15}
  ~ gain_item("Bounty Intel")
  情报 (qíngbào) — intelligence
* [[+10 credits] Sell your own intel to other hunters]
  ~ gain_credits(10)
  卖 (mài) — sell
