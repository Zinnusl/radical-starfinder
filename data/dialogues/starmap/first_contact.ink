=== EVENT_FIRST_CONTACT ===
# id: 35
# title: First Contact
# chinese_title: 第一次接触
# category: AlienContact
# mode: starmap

An alien vessel of unknown design approaches. It broadcasts a complex signal — possibly a greeting, possibly a warning.

* [Attempt communication using universal constants]
  ~ gain_credits(20)
  沟通 (gōutōng) — communicate
* [Offer a gift of fuel as a peace gesture] {requires: fuel >= 3}
  ~ fuel_and_credits(-3, 30)
  和平 (hépíng) — peace
* [Power weapons and raise shields]
  ~ start_combat(2)
  防御 (fángyù) — defense
* [Retreat slowly — don't provoke them]
  ~ lose_fuel(2)
  后退 (hòutuì) — back away
