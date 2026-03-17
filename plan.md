# Radical Roguelike Roadmap

## Current Status
The single-player game is up to date through Phase 32 deep roguelike overhaul.
Implemented systems now include dungeon crawling, pinyin combat, radical forging, equipment and items, codex and achievements, daily/endless modes, tutorial/settings polish, environmental hazards, boss variety, mystery item identities, inventory/help overlays, script seals, a deity/piety system, polymorph forms, dipping interactions, enemy component shields, crate pushing, bridge building, wall digging, cracked-wall secret rooms, puzzle niches, tighter resource economy with floor-profile-driven scarcity, six distinct enemy AI behaviors, companion XP/leveling with scaled passives, 20 player classes with Dark Souls-style lore, cursed/blessed item states, a spirit/hunger clock, monster radical actions derived from hanzi semantic components, shopkeeper theft with escalating consequences, mimics and hidden trap tiles, combo chains with damage multipliers, floor-scaled sentence construction boss phases, and a paginated run journal with post-mortem death screen.

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

### Phase 30: Resource Pressure Tuning and Enemy AI

#### Resource Pressure
- Enemy gold income reduced across the board: normal kills yield `3+floor` (was `5+floor*2`), elites `8+floor*2` (was `15+floor*3`), boss gold cut ~30%.
- New `FloorProfile::Drought` added (10% chance on floors 3+): 0.3× gold multiplier and 0% radical drops.
- Floor profile weights rebalanced: Normal 45%, Famine 20%, RadicalRich 15%, Siege 10%, Drought 10%.
- Radical drops now probabilistic per floor profile (Normal 80%, Famine 50%, RadicalRich 100%, Siege 80%, Drought 0%) instead of guaranteed.
- Shop prices increased: heal `20+floor*4`, radical `12+floor*2`, equipment `30+floor*6`, consumable `15+floor*3`.
- Equipment drop rate halved from 10% to 5% (bosses remain 60%).
- Quest rewards reduced ~30% across all quest types to remove the pure-bonus economy problem.

#### Enemy AI
- Three new `AiBehavior` variants: Sentinel (holds position, engages only when adjacent), Kiter (retreats when close, advances when far, holds at medium range), Pack (chases only with 2+ nearby allies or when adjacent).
- AI distribution rebalanced across six behaviors: Chase 44%, Ambush 12.5%, Retreat 12.5%, Sentinel 12.5%, Kiter 12.5%, Pack 6.25%.
- Nearby-ally computation added to the enemy movement loop for pack coordination.
- Comprehensive tests for all six AI behaviors and all five floor profiles.

### Phase 31: Companion Depth

#### Companion XP and Leveling
- Companions now earn XP from kills (2 normal, 3 elite, 5 boss) and quest completions (10 per quest).
- Three-tier leveling: L1 at 0 XP, L2 at 30 XP, L3 at 80 XP.
- Companion XP resets when recruiting a new companion.

#### Level-Scaled Passive Perks
- Teacher: L1 meaning hint → L2 adds pinyin → L3 adds radical breakdown.
- Monk: L1 heal 1 HP/floor → L2 heal 2 HP/floor → L3 heal 2 HP + cure one negative status.
- Merchant: L1 20% shop discount → L2 25% discount → L3 25% discount + one shop item reroll per floor (R key).
- Guard: L1 block 1 hit/fight → L2 block 1 + 50% chance of a second → L3 block 2 hits/fight guaranteed.

#### Contextual Exploration Hints
- Companions now comment on visible tiles during exploration (not just combat).
- Teacher: notices forges when player has radicals.
- Monk: notices shrines/altars when player HP is low.
- Merchant: notices chests on radical-rich floors.
- Guard: warns when 3+ alert enemies are closing in.

#### HUD and Rendering
- Companion level displayed in the HUD sidebar and inventory overlay.
- Shop discount now uses the level-aware Merchant perk value instead of a hardcoded 20%.
- Shop hint bar shows "R=reroll" when Merchant is L3.

### Phase 32: Deep Roguelike Overhaul

#### Talent Tree Removal
- Removed talent tree entirely for a true roguelike with no meta-progression between runs.

#### Class Expansion
- Expanded from 3 to 20 player classes, each with unique lore, icon, color, HP range, inventory size, and starting item/equipment loadouts.
- Paginated cursor-based class selection screen.

#### Monster Radical Action System
- 18 `RadicalAction` variants mapped from hanzi semantic components (火→FireBreath, 水→WaterShield, 心→SelfHeal, etc.).
- Each monster gains abilities derived from the radicals in its hanzi representation.
- Actions modify combat: armor, dodge, multiply, heal, disarm, root, charm, and more.
- Look mode displays each enemy's radical abilities.

#### Cursed/Blessed Item States
- Items and equipment can be Cursed (💀), Blessed (✨), or Normal.
- Cursed equipment cannot be replaced; cursed consumables have weakened effects.
- Blessed items have enhanced effects.
- Altar purification removes curses from all equipped items.

#### Spirit/Hunger Clock
- Spirit resource (100 max) decreases by 1 per move; starvation damage at 0.
- RiceBall item restores spirit; InkWell gives +25 spirit.
- Spirit bar displayed on HUD.

#### Shopkeeper Theft
- 'G' key to attempt theft with probability system (40% base, class bonuses, catch penalties).
- Caught: damage, shop ban, 15% price surcharge per catch.
- Ban resets on new floor.

#### Mimics and Trap Tiles
- Hidden trap tiles look like normal floor; 3 types: poison, teleport, alarm.
- Chests have 10% mimic chance spawning tougher enemies.

#### Combo Chains
- Sequential correct pinyin answers build a streak through 6 tiers (Good→Radical).
- Damage multiplier scales from 1.0× to 2.0×.
- HUD shows streak badge with tier name and color.

#### Sentence Construction Boss Phase Expansion
- Expanded from 10 to 30 sentences across 3 difficulty tiers (easy, medium, hard).
- Floor-scaled sentence selection: easy for early floors, all tiers for late floors.
- New GatekeeperSeal mode: Gatekeeper boss triggers a seal phase at half HP; failure damages the player.
- BonusGold rewards now scale with floor number.

#### Run Journal and Post-Mortem
- RunEvent system tracks floor entries, kills, boss kills, spell forges, items found, and death cause.
- Paginated death screen: summary page + floor-by-floor event log.

## Proposed Next Improvements

Goal: continue deepening single-player runs with systemic, NetHack-like interactions that build on the Phase 32 overhaul.

### Candidate Phase 33 Tracks
- Alignment arcs and deity synergies
  - Track broader playstyle patterns across deity choices and reward them with small run-defining perks.
  - This adds replayability using the existing piety and altar systems instead of introducing an all-new subsystem.
- Quest chains with dungeon impact
  - Let quests spawn follow-up objectives, alter floor generation, and reward the player with more than just gold.
  - This would make the current quest framework feel more like an emergent campaign.
- Advanced puzzle room patterns
  - Spike bridges, oil-fire ambush caches, and seal-triggered vault traps.
  - With the 20-class system and radical actions, puzzle rooms can now interact with class abilities.

### Recommended Next Slice
- Best low-risk / high-leverage start: alignment arcs or quest chains.
- Best follow-up: advanced puzzle rooms that leverage class-specific interactions.

## Deferred / Future Work

### Multiplayer Co-op (Deferred)
- Remains blocked on a larger architecture pass and networking layer.
- Keep this out of active planning until the single-player roadmap stabilizes further.
