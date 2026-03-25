=== ELDER_BEING ===
# id: 28
# title: The Elder Being
# chinese_title: 古老存在
# category: Alien
# mode: dungeon

In a sealed observation chamber, you find a being suspended in stasis fluid. Its form defies easy description — too many angles, too few dimensions. Ancient monitoring equipment still tracks its vital signs. A placard reads: "Specimen acquired pre-Gap. Do not release."

* [Communicate through the stasis field] {requires: class == 2}
  ~ gain_radical("龙")
  "龙" (lóng) — dragon, ancient power
* [Study the monitoring data]
  ~ gain_xp(25)
  "老" (lǎo) — old, ancient, venerable
* [Leave it in stasis — the warning is clear]
  ~ gain_xp(10)
  "危" (wēi) — danger, dangerous
* [Release the being]
  ~ start_fight
  "放" (fàng) — to release, to let go
