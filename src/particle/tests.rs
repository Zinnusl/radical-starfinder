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

