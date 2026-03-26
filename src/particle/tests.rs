use super::*;

fn make_particle(life: f64, decay: f64) -> Particle {
    Particle {
        x: 0.0, y: 0.0, vx: 1.0, vy: 0.0,
        life, decay, size: 2.0, r: 255, g: 255, b: 255,
    }
}

#[test]
fn particle_alive_when_life_positive() {
    assert!(make_particle(0.5, 0.01).alive());
    assert!(make_particle(0.001, 0.01).alive());
}

#[test]
fn particle_dead_when_life_zero_or_negative() {
    assert!(!make_particle(0.0, 0.01).alive());
    assert!(!make_particle(-0.1, 0.01).alive());
}

#[test]
fn tick_applies_velocity_gravity_and_decay() {
    let mut p = Particle {
        x: 10.0, y: 20.0, vx: 2.0, vy: -1.0,
        life: 1.0, decay: 0.05, size: 3.0, r: 0, g: 0, b: 0,
    };
    p.tick();
    assert!((p.x - 12.0).abs() < 1e-9, "x should move by vx");
    assert!((p.y - 19.0).abs() < 1e-9, "y should move by vy");
    assert!((p.vy - (-1.0 + 0.15)).abs() < 1e-9, "gravity applied to vy");
    assert!((p.life - 0.95).abs() < 1e-9, "life decayed by decay");
}

#[test]
fn particle_dies_after_enough_ticks() {
    let mut p = make_particle(0.1, 0.05);
    assert!(p.alive());
    for _ in 0..3 {
        p.tick();
    }
    assert!(!p.alive(), "particle should be dead after enough ticks");
}

#[test]
fn system_removes_dead_particles_on_tick() {
    let mut sys = ParticleSystem::new();
    sys.particles.push(make_particle(1.0, 0.01)); // will survive
    sys.particles.push(make_particle(0.01, 1.0)); // will die on first tick
    assert_eq!(sys.particles.len(), 2);
    sys.tick();
    assert_eq!(sys.particles.len(), 1, "dead particle should be removed");
}

#[test]
fn spawn_fire_creates_expected_particle_count() {
    let mut sys = ParticleSystem::new();
    let mut rng: u64 = 12345;
    sys.spawn_fire(100.0, 200.0, &mut rng);
    assert_eq!(sys.particles.len(), 30, "fire = 20 + 10 particles");
}

#[test]
fn spawn_heal_particles_move_upward() {
    let mut sys = ParticleSystem::new();
    let mut rng: u64 = 42;
    sys.spawn_heal(50.0, 50.0, &mut rng);
    assert_eq!(sys.particles.len(), 15);
    for p in &sys.particles {
        assert!(p.vy < 0.0, "heal particles should have negative vy (upward)");
    }
}

#[test]
fn rng_f64_stays_in_unit_range() {
    let mut state: u64 = 98765;
    for _ in 0..1000 {
        let val = rng_f64(&mut state);
        assert!(val >= 0.0 && val < 1.0, "rng value {val} out of [0,1)");
    }
}

#[test]
fn rng_f64_advances_state() {
    let mut state: u64 = 12345;
    let before = state;
    rng_f64(&mut state);
    assert_ne!(state, before, "rng should mutate state");
}

// ── spawn_shield ────────────────────────────────────────────────────────

#[test]
fn spawn_shield_creates_particles() {
    let mut sys = ParticleSystem::new();
    let mut rng: u64 = 100;
    sys.spawn_shield(50.0, 50.0, &mut rng);
    assert_eq!(sys.particles.len(), 15, "shield = 15 blue particles");
}

#[test]
fn spawn_shield_uses_blue_tones() {
    let mut sys = ParticleSystem::new();
    let mut rng: u64 = 100;
    sys.spawn_shield(50.0, 50.0, &mut rng);
    for p in &sys.particles {
        assert_eq!((p.r, p.g, p.b), (80, 160, 255));
    }
}

// ── spawn_kill ──────────────────────────────────────────────────────────

#[test]
fn spawn_kill_creates_particles() {
    let mut sys = ParticleSystem::new();
    let mut rng: u64 = 200;
    sys.spawn_kill(10.0, 20.0, &mut rng);
    assert_eq!(sys.particles.len(), 35, "kill = 25 + 10 particles");
}

// ── spawn_chest ─────────────────────────────────────────────────────────

#[test]
fn spawn_chest_creates_particles() {
    let mut sys = ParticleSystem::new();
    let mut rng: u64 = 300;
    sys.spawn_chest(0.0, 0.0, &mut rng);
    assert_eq!(sys.particles.len(), 26, "chest = 18 + 8 particles");
}

#[test]
fn spawn_chest_uses_gold_tones() {
    let mut sys = ParticleSystem::new();
    let mut rng: u64 = 300;
    sys.spawn_chest(0.0, 0.0, &mut rng);
    // First 18 particles are (220, 180, 40), next 8 are (255, 220, 100)
    for p in &sys.particles[..18] {
        assert_eq!((p.r, p.g, p.b), (220, 180, 40));
    }
    for p in &sys.particles[18..] {
        assert_eq!((p.r, p.g, p.b), (255, 220, 100));
    }
}

// ── spawn_damage ────────────────────────────────────────────────────────

#[test]
fn spawn_damage_creates_particles() {
    let mut sys = ParticleSystem::new();
    let mut rng: u64 = 400;
    sys.spawn_damage(5.0, 5.0, &mut rng);
    assert_eq!(sys.particles.len(), 12, "damage = 12 red particles");
}

#[test]
fn spawn_damage_uses_red() {
    let mut sys = ParticleSystem::new();
    let mut rng: u64 = 400;
    sys.spawn_damage(5.0, 5.0, &mut rng);
    for p in &sys.particles {
        assert_eq!((p.r, p.g, p.b), (255, 50, 50));
    }
}

// ── spawn_stun ──────────────────────────────────────────────────────────

#[test]
fn spawn_stun_creates_particles() {
    let mut sys = ParticleSystem::new();
    let mut rng: u64 = 500;
    sys.spawn_stun(0.0, 0.0, &mut rng);
    assert_eq!(sys.particles.len(), 12, "stun = 12 yellow particles");
}

// ── spawn_drain ─────────────────────────────────────────────────────────

#[test]
fn spawn_drain_creates_particles() {
    let mut sys = ParticleSystem::new();
    let mut rng: u64 = 600;
    sys.spawn_drain(0.0, 0.0, &mut rng);
    assert_eq!(sys.particles.len(), 15, "drain = 15 purple particles");
}

#[test]
fn spawn_drain_uses_purple() {
    let mut sys = ParticleSystem::new();
    let mut rng: u64 = 600;
    sys.spawn_drain(0.0, 0.0, &mut rng);
    for p in &sys.particles {
        assert_eq!((p.r, p.g, p.b), (180, 60, 220));
    }
}

// ── spawn_teleport ──────────────────────────────────────────────────────

#[test]
fn spawn_teleport_creates_particles() {
    let mut sys = ParticleSystem::new();
    let mut rng: u64 = 700;
    sys.spawn_teleport(0.0, 0.0, &mut rng);
    assert_eq!(sys.particles.len(), 20, "teleport = 20 cyan particles");
}

// ── spawn_dig ───────────────────────────────────────────────────────────

#[test]
fn spawn_dig_creates_particles() {
    let mut sys = ParticleSystem::new();
    let mut rng: u64 = 800;
    sys.spawn_dig(0.0, 0.0, &mut rng);
    assert_eq!(sys.particles.len(), 28, "dig = 18 + 10 particles");
}

// ── spawn_bridge ────────────────────────────────────────────────────────

#[test]
fn spawn_bridge_creates_particles() {
    let mut sys = ParticleSystem::new();
    let mut rng: u64 = 900;
    sys.spawn_bridge(0.0, 0.0, &mut rng);
    assert_eq!(sys.particles.len(), 24, "bridge = 14 + 10 particles");
}

// ── spawn_streak ────────────────────────────────────────────────────────

#[test]
fn spawn_streak_creates_particles() {
    let mut sys = ParticleSystem::new();
    let mut rng: u64 = 1000;
    sys.spawn_streak(0.0, 0.0, &mut rng);
    assert_eq!(sys.particles.len(), 18, "streak = 18 particles");
}

#[test]
fn spawn_streak_particles_move_upward() {
    let mut sys = ParticleSystem::new();
    let mut rng: u64 = 1000;
    sys.spawn_streak(50.0, 50.0, &mut rng);
    for p in &sys.particles {
        assert!(p.vy < 0.0, "streak particles should move upward");
    }
}

#[test]
fn spawn_streak_particles_have_warm_colors() {
    let mut sys = ParticleSystem::new();
    let mut rng: u64 = 1000;
    sys.spawn_streak(0.0, 0.0, &mut rng);
    for p in &sys.particles {
        assert_eq!(p.r, 255);
        assert_eq!(p.b, 20);
        assert!(p.g >= 100 && p.g <= 180, "green channel {}", p.g);
    }
}

// ── spawn_synergy ───────────────────────────────────────────────────────

#[test]
fn spawn_synergy_creates_particles() {
    let mut sys = ParticleSystem::new();
    let mut rng: u64 = 1100;
    sys.spawn_synergy(0.0, 0.0, &mut rng);
    assert_eq!(sys.particles.len(), 22, "synergy = 14 + 8 particles");
}

// ── spawn_altar ─────────────────────────────────────────────────────────

#[test]
fn spawn_altar_creates_particles() {
    let mut sys = ParticleSystem::new();
    let mut rng: u64 = 1200;
    sys.spawn_altar(0.0, 0.0, &mut rng);
    assert_eq!(sys.particles.len(), 34, "altar = 22 + 12 particles");
}

// ── spawn_knockback_collision ───────────────────────────────────────────

#[test]
fn spawn_knockback_collision_creates_particles() {
    let mut sys = ParticleSystem::new();
    let mut rng: u64 = 1300;
    sys.spawn_knockback_collision(0.0, 0.0, &mut rng);
    assert_eq!(sys.particles.len(), 24, "knockback = 16 + 8 particles");
}

// ── spawn_chengyu ───────────────────────────────────────────────────────

#[test]
fn spawn_chengyu_creates_particles() {
    let mut sys = ParticleSystem::new();
    let mut rng: u64 = 1400;
    sys.spawn_chengyu(0.0, 0.0, &mut rng);
    assert_eq!(sys.particles.len(), 44, "chengyu = 24 + 12 + 8 particles");
}

// ── spawn_wuxing_effective ──────────────────────────────────────────────

#[test]
fn spawn_wuxing_effective_creates_particles() {
    let mut sys = ParticleSystem::new();
    let mut rng: u64 = 1500;
    sys.spawn_wuxing_effective(0.0, 0.0, &mut rng);
    assert_eq!(sys.particles.len(), 28, "wuxing = 18 + 10 particles");
}

// ── spawn_rain_drop ─────────────────────────────────────────────────────

#[test]
fn spawn_rain_drop_creates_one_particle() {
    let mut sys = ParticleSystem::new();
    let mut rng: u64 = 1600;
    sys.spawn_rain_drop(0.0, 0.0, &mut rng);
    assert_eq!(sys.particles.len(), 1);
}

#[test]
fn spawn_rain_drop_falls_downward() {
    let mut sys = ParticleSystem::new();
    let mut rng: u64 = 1600;
    sys.spawn_rain_drop(50.0, 50.0, &mut rng);
    let p = &sys.particles[0];
    assert!(p.vy > 0.0, "rain should fall down (positive vy)");
    assert!(p.vx < 0.0, "rain has slight leftward drift");
}

#[test]
fn spawn_rain_drop_uses_blue_tones() {
    let mut sys = ParticleSystem::new();
    let mut rng: u64 = 1600;
    sys.spawn_rain_drop(0.0, 0.0, &mut rng);
    let p = &sys.particles[0];
    assert_eq!((p.r, p.g, p.b), (100, 140, 220));
}

// ── spawn_fog_wisp ──────────────────────────────────────────────────────

#[test]
fn spawn_fog_wisp_creates_one_particle() {
    let mut sys = ParticleSystem::new();
    let mut rng: u64 = 1700;
    sys.spawn_fog_wisp(0.0, 0.0, &mut rng);
    assert_eq!(sys.particles.len(), 1);
}

#[test]
fn spawn_fog_wisp_drifts_upward() {
    let mut sys = ParticleSystem::new();
    let mut rng: u64 = 1700;
    sys.spawn_fog_wisp(0.0, 0.0, &mut rng);
    let p = &sys.particles[0];
    assert!(p.vy < 0.0, "fog wisps drift upward");
}

#[test]
fn spawn_fog_wisp_has_slow_decay() {
    let mut sys = ParticleSystem::new();
    let mut rng: u64 = 1700;
    sys.spawn_fog_wisp(0.0, 0.0, &mut rng);
    let p = &sys.particles[0];
    assert!(p.decay < 0.015, "fog wisps should decay slowly, got {}", p.decay);
}

#[test]
fn spawn_fog_wisp_is_large() {
    let mut sys = ParticleSystem::new();
    let mut rng: u64 = 1700;
    sys.spawn_fog_wisp(0.0, 0.0, &mut rng);
    let p = &sys.particles[0];
    assert!(p.size >= 4.0, "fog wisps should be large, got {}", p.size);
}

// ── spawn_sand_grain ────────────────────────────────────────────────────

#[test]
fn spawn_sand_grain_creates_one_particle() {
    let mut sys = ParticleSystem::new();
    let mut rng: u64 = 1800;
    sys.spawn_sand_grain(0.0, 0.0, &mut rng);
    assert_eq!(sys.particles.len(), 1);
}

#[test]
fn spawn_sand_grain_moves_rightward() {
    let mut sys = ParticleSystem::new();
    let mut rng: u64 = 1800;
    sys.spawn_sand_grain(0.0, 0.0, &mut rng);
    let p = &sys.particles[0];
    assert!(p.vx > 0.0, "sand blows rightward");
}

#[test]
fn spawn_sand_grain_has_earthy_color() {
    let mut sys = ParticleSystem::new();
    let mut rng: u64 = 1800;
    sys.spawn_sand_grain(0.0, 0.0, &mut rng);
    let p = &sys.particles[0];
    assert_eq!((p.r, p.g, p.b), (200, 180, 120));
}

// ── spawn_ink_mote ──────────────────────────────────────────────────────

#[test]
fn spawn_ink_mote_creates_one_particle() {
    let mut sys = ParticleSystem::new();
    let mut rng: u64 = 1900;
    sys.spawn_ink_mote(0.0, 0.0, &mut rng);
    assert_eq!(sys.particles.len(), 1);
}

#[test]
fn spawn_ink_mote_drifts_upward() {
    let mut sys = ParticleSystem::new();
    let mut rng: u64 = 1900;
    sys.spawn_ink_mote(0.0, 0.0, &mut rng);
    let p = &sys.particles[0];
    assert!(p.vy < 0.0, "ink motes drift upward");
}

#[test]
fn spawn_ink_mote_is_dark() {
    let mut sys = ParticleSystem::new();
    let mut rng: u64 = 1900;
    sys.spawn_ink_mote(0.0, 0.0, &mut rng);
    let p = &sys.particles[0];
    assert_eq!((p.r, p.g, p.b), (40, 20, 60));
}

// ── spawn_poison (dead_code but still callable) ─────────────────────────

#[test]
fn spawn_poison_creates_particles() {
    let mut sys = ParticleSystem::new();
    let mut rng: u64 = 2000;
    sys.spawn_poison(0.0, 0.0, &mut rng);
    assert_eq!(sys.particles.len(), 10, "poison = 10 green particles");
}

// ── burst helper – particle properties ──────────────────────────────────

#[test]
fn burst_particles_start_alive() {
    let mut sys = ParticleSystem::new();
    let mut rng: u64 = 42;
    sys.spawn_fire(0.0, 0.0, &mut rng);
    for p in &sys.particles {
        assert!(p.alive(), "freshly spawned particle should be alive");
        assert!((p.life - 1.0).abs() < f64::EPSILON);
    }
}

#[test]
fn burst_particles_have_positive_size() {
    let mut sys = ParticleSystem::new();
    let mut rng: u64 = 42;
    sys.spawn_shield(0.0, 0.0, &mut rng);
    for p in &sys.particles {
        assert!(p.size > 0.0, "particles should have positive size");
    }
}

#[test]
fn burst_particles_have_positive_decay() {
    let mut sys = ParticleSystem::new();
    let mut rng: u64 = 42;
    sys.spawn_drain(0.0, 0.0, &mut rng);
    for p in &sys.particles {
        assert!(p.decay > 0.0, "particles should decay over time");
    }
}

// ── multiple spawn calls accumulate ─────────────────────────────────────

#[test]
fn multiple_spawns_accumulate_particles() {
    let mut sys = ParticleSystem::new();
    let mut rng: u64 = 42;
    sys.spawn_fire(0.0, 0.0, &mut rng);
    let after_fire = sys.particles.len();
    sys.spawn_heal(0.0, 0.0, &mut rng);
    assert_eq!(sys.particles.len(), after_fire + 15);
    sys.spawn_rain_drop(0.0, 0.0, &mut rng);
    assert_eq!(sys.particles.len(), after_fire + 15 + 1);
}

