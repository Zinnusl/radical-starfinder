=== EVENT_ANCIENT_TERMINAL ===
# id: 45
# title: Ancient Terminal
# chinese_title: 古代终端
# category: LanguageChallenge
# mode: starmap

You discover an operational terminal in an abandoned outpost. Its interface displays Chinese characters — an ancient Earth colony, perhaps. It seems to require a passphrase.

* [Enter 'open' (开) to unlock] {requires: radical == "开"}
  ~ gain_credits(30)
  开 (kāi) — open
* [Try to brute-force the terminal]
  ~ gain_credits(10)
  破解 (pòjiě) — crack
* [Copy the data and decrypt it later]
  ~ gain_radical("开")
  复制 (fùzhì) — copy
