=== MAGNETIC_ANOMALY ===
# id: 44
# title: The Magnetic Anomaly
# chinese_title: 磁场异常
# category: Hazard
# mode: dungeon

Your equipment goes haywire as you enter a zone of intense magnetic flux. Metal objects pull toward the walls, your navigation tools spin uselessly, and your suit's electronics flicker. In the center of the anomaly, a lodestone-like object hovers, crackling with power.

* [Fight through the magnetic pull to reach the object] {requires: hp >= 10}
  ~ gain_radical("金")
  "金" (jīn) — metal, gold
* [Demagnetize your equipment and retreat]
  ~ gain_xp(15)
  "退" (tuì) — to retreat, to withdraw
* [Use the anomaly to recharge your systems]
  ~ heal(15)
  "充" (chōng) — to charge, to fill
