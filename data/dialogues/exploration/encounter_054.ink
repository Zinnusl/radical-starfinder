=== CIPHER_DOOR ===
# id: 53
# title: The Cipher Door
# chinese_title: 密码门
# category: Puzzle
# mode: dungeon

A massive blast door blocks your path, its surface etched with Chinese characters arranged in concentric circles. A terminal beside it prompts for input. The characters seem to follow a pattern — radicals combining to form compound words. The answer lies in understanding their structure.

* [Solve the radical combination puzzle]
  ~ gain_radical("门")
  "门" (mén) — door, gate, gateway
* [Brute-force the terminal] {requires: class == 3}
  ~ gain_xp(20)
  "破" (pò) — to break, to crack
* [Look for a physical bypass]
  ~ gain_xp(15)
  "找" (zhǎo) — to find, to search
* [Blast through the door]
  ~ damage(5)
  "炸" (zhà) — to explode, to blast
