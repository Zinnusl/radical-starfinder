1. # Phase 27: Enemy Complexity & Physics
2. 
3. ## Problem
4. User Feedback: "The hanzi enemies with more life are boring, because you still just type their pinyin to kill them."
5. The game lacks physical interaction with the environment (sokoban, digging) which is a staple of NetHack-like depth.
6. 
7. ## Approach
8. 1.  **Decomposing Enemies (Shields)**: Complex characters (e.g., Ming) are protected by their components (Sun, Moon). The player must identify and type the components to break the armor before damaging the main health. This teaches character composition.
9. 2.  **Physics Interaction (Pushing)**: Allow pushing Crates. Pushing a crate into Water creates a Bridge.
10. 3.  **Digging**: Allow destroying walls with Pickaxe or Stone Form.
11. 
12. ---
13. 
14. ## Todos
15. 
16. ### 1. Decomposing Enemies (src/enemy.rs, src/game.rs)
17. - [ ] Add components: Vec<&'static str> to Enemy struct.
18. - [ ] Define decomposition data for common enemies.
19. - [ ] Update Combat Logic: If enemy.components is not empty, the active target is the first component.
20. - [ ] Update Rendering: Draw the Shield Component overlay on the enemy.
21. 
22. ### 2. Physics: Pushing (src/game.rs)
23. - [ ] Update try_move: If moving into Crate -> Check tile behind crate.
24. - [ ] If empty/floor: Move crate, Move player.
25. - [ ] If Water: Crate becomes Bridge (Walkable), Move player.
26. - [ ] If Wall/Enemy: Blocked (kick sound?).
27. 
28. ### 3. Physics: Digging (src/game.rs, src/player.rs)
29. - [ ] New Item: Pickaxe.
30. - [ ] New Action: Bump-to-dig if holding Pickaxe/in Stone Form (with confirmation).
31. 
32. ### 4. Content Expansion
33. - [ ] Add Bridge tile type.
34. - [ ] Add Pickaxe to item pool.
35. - [ ] Add more complex enemies to dungeon/generation.rs.

