# Radical Roguelike Roadmap

## Current Status
The single-player game is up to date through the Phase 28 follow-through polish pass.
Implemented systems now include dungeon crawling, pinyin combat, radical forging, equipment and items, codex and achievements, daily/endless modes, tutorial/settings polish, environmental hazards, boss variety, talents, mystery item identities, inventory/help overlays, script seals, a deity/piety system, polymorph forms, dipping interactions, enemy component shields, crate pushing, bridge building, wall digging, cracked-wall secret rooms with tuned physics feedback, longer-lived message popups, and a 3-tile look/inspect mode.

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

## Deferred / Future Work

### Multiplayer Co-op (Deferred)
- Remains blocked on a larger architecture pass and networking layer.
- Keep this out of active planning until the single-player roadmap stabilizes further.
