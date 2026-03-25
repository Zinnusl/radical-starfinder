=== STAR_WHALE_CALF ===
# id: 34
# title: The Star Whale Calf
# chinese_title: 星鲸幼崽
# category: Alien
# mode: dungeon

A massive shadow drifts past the observation window — a juvenile star whale, its bioluminescent markings pulsing in distress. It has become entangled in derelict satellite debris. Its cries resonate through the hull, a deep thrumming that vibrates in your chest.

* [Suit up and cut it free from the debris] {requires: hp >= 15}
  ~ gain_radical("鸟")
  "鸟" (niǎo) — bird, flying creature. Star whales are the birds of space.
* [Use the station's tractor beam to help] {requires: class == 1}
  ~ gain_xp(30)
  "救" (jiù) — to rescue, to save
* [Signal for help from other ships]
  ~ gain_xp(15)
  "呼" (hū) — to call, to cry out
* [Record the whale's distress call for research]
  ~ gain_gold(15)
  "鲸" (jīng) — whale
