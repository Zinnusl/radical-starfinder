# Radical Roguelike Roadmap

## Current Status
The single-player game is up to date through the first Phase 29 environmental puzzle-room pass.
Implemented systems now include dungeon crawling, pinyin combat, radical forging, equipment and items, codex and achievements, daily/endless modes, tutorial/settings polish, environmental hazards, boss variety, talents, mystery item identities, inventory/help overlays, script seals, a deity/piety system, polymorph forms, dipping interactions, enemy component shields, crate pushing, bridge building, wall digging, cracked-wall secret rooms with tuned physics feedback, longer-lived message popups, a 3-tile look/inspect mode, and visible puzzle niches with brittle-wall vaults and deep-water bridge caches.

Multiplayer co-op remains deferred future work and is not part of the current single-player roadmap.

## Completed Roadmap

### Phases 7-13: Core Expansion
- Visual polish and juice: particles, screen shake, procedural music.
- Meta progression: achievements, codex, unlockable radical tiers.
- Gameplay depth: radical enchanting, room modifiers, spell combos.
- Language-learning features: listening mode, tone battles, contextual hints, sentence construction.
- RPG systems: companions, procedural quests, class specializations.
- Replayability and platform work: daily challenge, endless mode, touch controls, PWA/offline support.

### Phases 14-18: Onboarding, Exploration, and Boss Depth
- Interactive tutorial floor, settings menu, and general visual polish.
- Hazard interactions, elemental synergies, and destructible objects.
- Unique bosses plus the talent tree.
- Utility spell expansion and forge-character quest support.
- Blessing altars and exploration rewards.

### Phases 19-25: UX, Presentation, and Systemic Content
- Inventory overlay for radicals, spells, items, equipment, and effects.
- NetHack-style mystery item identities.
- Skip-floor hotkey for testing.
- Compound elite combat with syllable-chain sequencing.
- Beauty pass items: help overlay, clearer elite progress, better message colors.
- Tile visual refresh: stronger palette separation, depth, and animated accents.
- Script seal rooms with room-shaping triggers and clear visual hints.

### Phase 26: Gods, Polymorph, and Dipping
- Five deity alignments with altar-specific piety, offerings, and prayer rewards.
- Temporary polymorph forms for alternate combat/exploration bonuses.
- Dipping system for coating weapons with potion-based effects.

### Phase 27: Enemy Complexity and Physics
- Hanzi component shields on complex enemies, requiring shield-break typing before HP damage.
- Crate pushing and crate-into-water bridge building.
- Digging through walls with the Iron Pickaxe.
- Supporting UI/rendering updates and warning cleanup around the new systems.

### Phase 28: Secrets and Environmental Follow-Through
- Cracked-wall secret rooms now hide treasure, altars, shrines, or forges behind diggable terrain.
- Hazard and crate placement now more reliably create bridge-building opportunities.
- Digging and bridge creation now have stronger sound, particles, shake, and clearer messages.
- Automated generation tests now guard secret-room and bridge-setup frequency across sample seeds.

### Post-Phase 28 UX Follow-Through
- Message popups now linger longer across text speeds so important combat and interaction feedback is easier to read.
- A new `[V] Look` inspect mode lets the player examine visible enemies and nearby terrain within three tiles.

### Phase 29: Environmental Puzzle Rooms (First Pass)
- Optional puzzle niches now appear in regular rooms as brittle-wall vaults and deep-water bridge caches.
- These rooms reuse the existing digging and crate-bridge verbs rather than adding a separate puzzle control scheme.
- Look text, tile art, and generation tests now telegraph and protect the new puzzle-room interactions.

## Proposed Next Improvements

Goal: deepen single-player runs with more systemic, NetHack-like interactions that build on the current quest, companion, deity, hazard, and enemy-complexity systems.

### Candidate Phase 29 Tracks
- Companion depth and contextual advice
  - Add companion XP, stronger passive perks, and occasional context-aware hints tied to nearby rooms, loot, or hazards.
  - This is the lowest-risk way to add more personality and moment-to-moment decision support.
- Resource pressure and scarcity tuning
  - Tighten gold/radical availability, vary floor loot profiles, and force harder shop and consumable trade-offs.
  - This would make the existing item, shop, and recovery systems matter more run-to-run.
- Enemy tactics and room-aware AI
  - Expand enemy behavior beyond basic alert pursuit with ranged retreat, corridor blocking, ambush, and room-modifier-aware tactics.
  - This builds directly on the current alert state, elite complexity, and room modifier systems.
- Environmental puzzle room expansion
  - Build on the shipped first pass with more patterns such as spike bridges, oil-fire caches, or seal-driven trap vaults.
  - This remains a good later force multiplier once the surrounding economy and combat pacing are tuned.
- Alignment arcs and deity synergies
  - Track broader playstyle patterns across deity choices and reward them with small run-defining perks.
  - This adds replayability using the existing piety and altar systems instead of introducing an all-new subsystem.
- Quest chains with dungeon impact
  - Let quests spawn follow-up objectives, alter floor generation, and reward the player with more than just gold.
  - This would make the current quest framework feel more like an emergent campaign.

### Recommended Next Slice
- Best low-risk / high-leverage starts: companion depth or resource pressure.
- Best follow-up once pacing feels tighter: enemy tactics.
- Best later force multiplier once those systems are stable: expand puzzle-room variants.

### Phase 29 Follow-ups
- Expand the first pass with a third or fourth puzzle pattern only after frequency and reward tuning still feel good in normal runs.
- Good future candidates are spike-bridge rooms, oil-fire ambush caches, and seal-triggered vault traps.

## Deferred / Future Work

### Multiplayer Co-op (Deferred)
- Remains blocked on a larger architecture pass and networking layer.
- Keep this out of active planning until the single-player roadmap stabilizes further.
