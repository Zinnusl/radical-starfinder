=== EVENT_TRAINING_EXERCISE ===
# id: 32
# title: Training Exercise
# chinese_title: 训练演习
# category: CrewEvent
# mode: starmap

During a quiet stretch of travel, you consider running combat drills. The crew could use the practice, but it will cost resources.

* [Full combat drill (costs fuel)] {requires: fuel >= 3}
  ~ hull_and_fuel(0, -3)
  训练 (xùnliàn) — training
* [Simulator exercises only]
  ~ heal_crew(3)
  模拟 (mónǐ) — simulate
* [Let the crew rest instead]
  ~ heal_crew(8)
  休息 (xiūxi) — rest
