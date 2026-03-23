//! Lightweight Canvas 2D particle system.

/// A single particle with position, velocity, color, and lifetime.
#[derive(Clone)]
pub struct Particle {
    pub x: f64,
    pub y: f64,
    pub vx: f64,
    pub vy: f64,
    pub life: f64,  // 0.0 .. 1.0 (1.0 = just spawned)
    pub decay: f64, // life lost per tick (higher = shorter-lived)
    pub size: f64,
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Particle {
    pub fn alive(&self) -> bool {
        self.life > 0.0
    }

    pub fn tick(&mut self) {
        self.x += self.vx;
        self.y += self.vy;
        self.vy += 0.15; // gravity
        self.life -= self.decay;
    }
}

/// The particle manager — holds all active particles and ticks/draws them.
pub struct ParticleSystem {
    pub particles: Vec<Particle>,
}

impl ParticleSystem {
    pub fn new() -> Self {
        Self {
            particles: Vec::new(),
        }
    }

    pub fn tick(&mut self) {
        for p in &mut self.particles {
            p.tick();
        }
        self.particles.retain(|p| p.alive());
    }

    /// Burst of particles at a screen position.
    fn burst(
        &mut self,
        x: f64,
        y: f64,
        count: usize,
        r: u8,
        g: u8,
        b: u8,
        speed: f64,
        rng: &mut u64,
    ) {
        for _ in 0..count {
            let angle = rng_f64(rng) * std::f64::consts::TAU;
            let spd = speed * (0.5 + rng_f64(rng) * 0.5);
            self.particles.push(Particle {
                x,
                y,
                vx: angle.cos() * spd,
                vy: angle.sin() * spd - 1.0,
                life: 1.0,
                decay: 0.02 + rng_f64(rng) * 0.03,
                size: 2.0 + rng_f64(rng) * 3.0,
                r,
                g,
                b,
            });
        }
    }

    // --- Effect spawners ---

    /// Fire spell particles (orange/red burst)
    pub fn spawn_fire(&mut self, x: f64, y: f64, rng: &mut u64) {
        self.burst(x, y, 20, 255, 120, 30, 3.0, rng);
        self.burst(x, y, 10, 255, 60, 10, 2.0, rng);
    }

    /// Heal effect (green upward particles)
    pub fn spawn_heal(&mut self, x: f64, y: f64, rng: &mut u64) {
        for _ in 0..15 {
            self.particles.push(Particle {
                x: x + (rng_f64(rng) - 0.5) * 20.0,
                y,
                vx: (rng_f64(rng) - 0.5) * 0.5,
                vy: -(1.5 + rng_f64(rng) * 2.0),
                life: 1.0,
                decay: 0.02 + rng_f64(rng) * 0.02,
                size: 2.0 + rng_f64(rng) * 2.0,
                r: 60,
                g: 220,
                b: 80,
            });
        }
    }

    /// Shield effect (blue shimmer ring)
    pub fn spawn_shield(&mut self, x: f64, y: f64, rng: &mut u64) {
        self.burst(x, y, 15, 80, 160, 255, 2.5, rng);
    }

    /// Kill sparkle (white/yellow burst)
    pub fn spawn_kill(&mut self, x: f64, y: f64, rng: &mut u64) {
        self.burst(x, y, 25, 255, 255, 200, 3.5, rng);
        self.burst(x, y, 10, 255, 200, 50, 2.0, rng);
    }

    /// Chest open (gold burst)
    pub fn spawn_chest(&mut self, x: f64, y: f64, rng: &mut u64) {
        self.burst(x, y, 18, 220, 180, 40, 2.5, rng);
        self.burst(x, y, 8, 255, 220, 100, 1.5, rng);
    }

    /// Damage taken (red burst from player)
    pub fn spawn_damage(&mut self, x: f64, y: f64, rng: &mut u64) {
        self.burst(x, y, 12, 255, 50, 50, 2.5, rng);
    }

    /// Stun effect (yellow stars)
    pub fn spawn_stun(&mut self, x: f64, y: f64, rng: &mut u64) {
        self.burst(x, y, 12, 255, 220, 50, 2.0, rng);
    }

    /// Drain effect (purple)
    pub fn spawn_drain(&mut self, x: f64, y: f64, rng: &mut u64) {
        self.burst(x, y, 15, 180, 60, 220, 2.5, rng);
    }

    /// Poison cloud (green)
    #[allow(dead_code)]
    pub fn spawn_poison(&mut self, x: f64, y: f64, rng: &mut u64) {
        self.burst(x, y, 10, 80, 200, 60, 1.5, rng);
    }

    /// Teleport flash (cyan)
    pub fn spawn_teleport(&mut self, x: f64, y: f64, rng: &mut u64) {
        self.burst(x, y, 20, 100, 220, 255, 4.0, rng);
    }

    /// Digging debris (stone and dust)
    pub fn spawn_dig(&mut self, x: f64, y: f64, rng: &mut u64) {
        self.burst(x, y, 18, 140, 120, 100, 2.6, rng);
        self.burst(x, y, 10, 96, 86, 82, 1.8, rng);
    }

    /// Splash and splinters when a bridge is formed.
    pub fn spawn_bridge(&mut self, x: f64, y: f64, rng: &mut u64) {
        self.burst(x, y, 14, 120, 180, 255, 2.2, rng);
        self.burst(x, y, 10, 156, 102, 58, 1.9, rng);
    }

    pub fn spawn_streak(&mut self, x: f64, y: f64, rng: &mut u64) {
        for _ in 0..18 {
            self.particles.push(Particle {
                x: x + (rng_f64(rng) - 0.5) * 16.0,
                y,
                vx: (rng_f64(rng) - 0.5) * 1.2,
                vy: -(2.5 + rng_f64(rng) * 2.5),
                life: 1.0,
                decay: 0.015 + rng_f64(rng) * 0.02,
                size: 2.0 + rng_f64(rng) * 2.5,
                r: 255,
                g: (100.0 + rng_f64(rng) * 80.0) as u8,
                b: 20,
            });
        }
    }

    pub fn spawn_synergy(&mut self, x: f64, y: f64, rng: &mut u64) {
        self.burst(x, y, 14, 255, 215, 0, 2.0, rng);
        self.burst(x, y, 8, 255, 240, 100, 1.2, rng);
    }

    pub fn spawn_altar(&mut self, x: f64, y: f64, rng: &mut u64) {
        self.burst(x, y, 22, 200, 160, 255, 3.0, rng);
        self.burst(x, y, 12, 255, 200, 255, 2.0, rng);
    }

    pub fn spawn_knockback_collision(&mut self, x: f64, y: f64, rng: &mut u64) {
        self.burst(x, y, 16, 200, 180, 160, 3.5, rng);
        self.burst(x, y, 8, 255, 255, 200, 2.0, rng);
    }

    pub fn spawn_chengyu(&mut self, x: f64, y: f64, rng: &mut u64) {
        self.burst(x, y, 24, 255, 215, 0, 3.5, rng);
        self.burst(x, y, 12, 255, 100, 50, 2.5, rng);
        self.burst(x, y, 8, 255, 255, 180, 1.5, rng);
    }

    pub fn spawn_wuxing_effective(&mut self, x: f64, y: f64, rng: &mut u64) {
        self.burst(x, y, 18, 120, 255, 120, 3.0, rng);
        self.burst(x, y, 10, 255, 255, 100, 2.0, rng);
    }

    pub fn spawn_rain_drop(&mut self, x: f64, y: f64, rng: &mut u64) {
        self.particles.push(Particle {
            x: x + (rng_f64(rng) - 0.5) * 6.0,
            y,
            vx: -0.3,
            vy: 3.0 + rng_f64(rng) * 2.0,
            life: 1.0,
            decay: 0.06 + rng_f64(rng) * 0.04,
            size: 1.0 + rng_f64(rng),
            r: 100,
            g: 140,
            b: 220,
        });
    }

    pub fn spawn_fog_wisp(&mut self, x: f64, y: f64, rng: &mut u64) {
        self.particles.push(Particle {
            x,
            y,
            vx: (rng_f64(rng) - 0.5) * 0.4,
            vy: -(0.2 + rng_f64(rng) * 0.3),
            life: 1.0,
            decay: 0.008 + rng_f64(rng) * 0.006,
            size: 4.0 + rng_f64(rng) * 4.0,
            r: 180,
            g: 180,
            b: 200,
        });
    }

    pub fn spawn_sand_grain(&mut self, x: f64, y: f64, rng: &mut u64) {
        self.particles.push(Particle {
            x,
            y,
            vx: 1.5 + rng_f64(rng) * 2.0,
            vy: (rng_f64(rng) - 0.5) * 0.8,
            life: 1.0,
            decay: 0.04 + rng_f64(rng) * 0.03,
            size: 1.0 + rng_f64(rng) * 1.5,
            r: 200,
            g: 180,
            b: 120,
        });
    }

    pub fn spawn_ink_mote(&mut self, x: f64, y: f64, rng: &mut u64) {
        self.particles.push(Particle {
            x,
            y,
            vx: (rng_f64(rng) - 0.5) * 0.6,
            vy: -(0.5 + rng_f64(rng) * 1.0),
            life: 1.0,
            decay: 0.012 + rng_f64(rng) * 0.008,
            size: 2.0 + rng_f64(rng) * 2.5,
            r: 40,
            g: 20,
            b: 60,
        });
    }
}

/// Simple xorshift rng returning f64 in [0,1).
fn rng_f64(state: &mut u64) -> f64 {
    let mut x = *state;
    x ^= x << 13;
    x ^= x >> 7;
    x ^= x << 17;
    *state = x;
    (x & 0xFFFF) as f64 / 65536.0
}

#[cfg(test)]
mod tests {
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
}
