=== EVENT_CALLIGRAPHY_CONTEST ===
# id: 48
# title: Calligraphy Contest
# chinese_title: 书法比赛
# category: LanguageChallenge
# mode: starmap

A cultured space station hosts a calligraphy contest among travelers. The prize pool is generous, and the challenge: write a character from memory.

* [Enter the contest (need a radical to compete)] {requires: radical == "人"}
  ~ gain_credits(40)
  比赛 (bǐsài) — contest
* [Watch and learn from the contestants]
  ~ gain_radical("文")
  学习 (xuéxí) — learn
* [Bet on the winner] {requires: credits >= 10}
  ~ gain_credits(15)
  下注 (xiàzhù) — bet
