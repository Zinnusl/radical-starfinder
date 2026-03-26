//! Procedural audio using Web Audio API.
//! All sounds are synthesized — no audio files needed.
//!
//! Uses layered oscillators, ADSR envelopes, and biquad filters for
//! richer, more atmospheric sound design.

use web_sys::{
    AudioContext, AudioNode, BiquadFilterNode, BiquadFilterType, GainNode, OscillatorNode,
    OscillatorType,
};

/// Current music mood.
#[derive(Clone, Copy, PartialEq)]
pub enum MusicMood {
    Explore,
    Combat,
    Boss,
    Silent,
}

/// ADSR envelope parameters (all in seconds).
struct Adsr {
    attack: f64,
    decay: f64,
    sustain: f32, // sustain level (0.0..1.0 of peak)
    release: f64,
}

impl Adsr {
    fn quick() -> Self {
        Self { attack: 0.01, decay: 0.04, sustain: 0.6, release: 0.08 }
    }
    fn medium() -> Self {
        Self { attack: 0.02, decay: 0.06, sustain: 0.5, release: 0.15 }
    }
    fn soft() -> Self {
        Self { attack: 0.05, decay: 0.1, sustain: 0.4, release: 0.25 }
    }
    fn percussive() -> Self {
        Self { attack: 0.005, decay: 0.08, sustain: 0.1, release: 0.1 }
    }
    #[allow(dead_code)]
    fn pad_env() -> Self {
        Self { attack: 0.3, decay: 0.2, sustain: 0.7, release: 0.5 }
    }
}

pub struct Audio {
    ctx: AudioContext,
    /// Time when next ambient phrase should play.
    music_next: f64,
    /// Current music mood.
    music_mood: MusicMood,
    /// Music volume as 0.0..1.0 multiplier.
    music_volume: f32,
    /// SFX volume as 0.0..1.0 multiplier.
    sfx_volume: f32,
}

impl Audio {
    pub fn new() -> Option<Self> {
        AudioContext::new().ok().map(|ctx| Self {
            music_next: 0.0,
            music_mood: MusicMood::Explore,
            music_volume: 1.0,
            sfx_volume: 1.0,
            ctx,
        })
    }

    /// Resume audio context (required after user interaction on some browsers).
    pub fn resume(&self) {
        let _ = self.ctx.resume();
    }

    /// Set the music mood (changes ambient music character).
    pub fn set_mood(&mut self, mood: MusicMood) {
        if self.music_mood != mood {
            self.music_mood = mood;
            self.music_next = 0.0;
        }
    }

    pub fn set_music_volume(&mut self, volume_percent: u8) {
        self.music_volume = (volume_percent.min(100) as f32) / 100.0;
    }

    pub fn set_sfx_volume(&mut self, volume_percent: u8) {
        self.sfx_volume = (volume_percent.min(100) as f32) / 100.0;
    }

    // ── helpers ──────────────────────────────────────────────────────

    fn now(&self) -> f64 {
        self.ctx.current_time()
    }

    fn osc(&self) -> Option<OscillatorNode> {
        self.ctx.create_oscillator().ok()
    }

    fn gain(&self) -> Option<GainNode> {
        self.ctx.create_gain().ok()
    }

    fn filter(&self) -> Option<BiquadFilterNode> {
        self.ctx.create_biquad_filter().ok()
    }

    fn dest(&self) -> AudioNode {
        self.ctx.destination().into()
    }

    /// Apply an ADSR envelope to a gain node.  Returns total duration.
    fn apply_adsr(&self, g: &GainNode, peak: f32, env: &Adsr, t0: f64) -> f64 {
        let sustain_dur = 0.0_f64; // for short SFX the sustain phase is ~0
        let total = env.attack + env.decay + sustain_dur + env.release;
        g.gain().set_value(0.0);
        let _ = g.gain().linear_ramp_to_value_at_time(peak, t0 + env.attack);
        let _ = g.gain().linear_ramp_to_value_at_time(
            peak * env.sustain,
            t0 + env.attack + env.decay,
        );
        let _ = g.gain().linear_ramp_to_value_at_time(
            peak * env.sustain,
            t0 + env.attack + env.decay + sustain_dur,
        );
        let _ = g.gain().linear_ramp_to_value_at_time(0.0, t0 + total);
        total
    }

    /// Create a single oscillator with ADSR routed to destination (or a target node).
    /// Returns the total duration of the sound.
    #[allow(clippy::too_many_arguments)]
    fn voice(
        &self,
        freq: f32,
        detune: f32,
        wave: OscillatorType,
        vol: f32,
        env: &Adsr,
        target: &AudioNode,
        t0: f64,
    ) -> f64 {
        let osc = match self.osc() {
            Some(o) => o,
            None => return 0.0,
        };
        let g = match self.gain() {
            Some(g) => g,
            None => return 0.0,
        };
        osc.set_type(wave);
        osc.frequency().set_value(freq);
        osc.detune().set_value(detune);
        let dur = self.apply_adsr(&g, vol, env, t0);
        let _ = osc.connect_with_audio_node(&g);
        let _ = g.connect_with_audio_node(target);
        let _ = osc.start_with_when(t0);
        let _ = osc.stop_with_when(t0 + dur + 0.05);
        dur
    }

    /// Two detuned oscillators layered for warmth.
    fn layered_voice(
        &self,
        freq: f32,
        wave: OscillatorType,
        vol: f32,
        env: &Adsr,
        target: &AudioNode,
        t0: f64,
    ) -> f64 {
        let d1 = self.voice(freq, -8.0, wave, vol * 0.6, env, target, t0);
        let d2 = self.voice(freq, 8.0, wave, vol * 0.6, env, target, t0);
        d1.max(d2)
    }

    /// Tone with a lowpass filter.
    fn filtered_tone(
        &self,
        freq: f32,
        wave: OscillatorType,
        vol: f32,
        env: &Adsr,
        cutoff: f32,
        filter_type: BiquadFilterType,
    ) {
        let filt = match self.filter() {
            Some(f) => f,
            None => return,
        };
        filt.set_type(filter_type);
        filt.frequency().set_value(cutoff);
        filt.q().set_value(1.0);
        let dest = self.dest();
        let _ = filt.connect_with_audio_node(&dest);
        let t0 = self.now();
        let filt_node: AudioNode = filt.into();
        self.layered_voice(freq, wave, vol, env, &filt_node, t0);
    }

    /// Rising sweep: frequency ramps from `f_start` to `f_end`.
    fn sweep(
        &self,
        f_start: f32,
        f_end: f32,
        duration: f64,
        wave: OscillatorType,
        vol: f32,
    ) {
        let vol = vol * self.sfx_volume;
        if vol <= 0.0 { return; }
        let osc = match self.osc() { Some(o) => o, None => return };
        let g = match self.gain() { Some(g) => g, None => return };
        let t0 = self.now();
        osc.set_type(wave);
        osc.frequency().set_value(f_start);
        let _ = osc.frequency().linear_ramp_to_value_at_time(f_end, t0 + duration);
        g.gain().set_value(vol);
        let _ = g.gain().linear_ramp_to_value_at_time(0.0, t0 + duration);
        let dest = self.dest();
        let _ = osc.connect_with_audio_node(&g);
        let _ = g.connect_with_audio_node(&dest);
        let _ = osc.start();
        let _ = osc.stop_with_when(t0 + duration + 0.02);
    }

    /// Simple tone (kept for backward compat and simple cases).
    fn tone(&self, freq: f32, duration: f64, volume: f32, wave: OscillatorType) {
        let volume = volume * self.sfx_volume;
        if volume <= 0.0 { return; }
        let osc = match self.osc() { Some(o) => o, None => return };
        let g = match self.gain() { Some(g) => g, None => return };
        osc.set_type(wave);
        osc.frequency().set_value(freq);
        g.gain().set_value(volume);
        let t0 = self.now();
        let _ = g.gain().linear_ramp_to_value_at_time(0.0, t0 + duration);
        let dest = self.dest();
        let _ = osc.connect_with_audio_node(&g);
        let _ = g.connect_with_audio_node(&dest);
        let _ = osc.start();
        let _ = osc.stop_with_when(t0 + duration);
    }

    /// Scheduled tone that starts at a given offset from now.
    fn tone_at(
        &self,
        freq: f32,
        duration: f64,
        volume: f32,
        wave: OscillatorType,
        delay: f64,
    ) {
        let volume = volume * self.sfx_volume;
        if volume <= 0.0 { return; }
        let osc = match self.osc() { Some(o) => o, None => return };
        let g = match self.gain() { Some(g) => g, None => return };
        let t0 = self.now() + delay;
        osc.set_type(wave);
        osc.frequency().set_value(freq);
        g.gain().set_value(0.0);
        let _ = g.gain().set_value_at_time(volume, t0);
        let _ = g.gain().linear_ramp_to_value_at_time(0.0, t0 + duration);
        let dest = self.dest();
        let _ = osc.connect_with_audio_node(&g);
        let _ = g.connect_with_audio_node(&dest);
        let _ = osc.start_with_when(t0);
        let _ = osc.stop_with_when(t0 + duration + 0.02);
    }

    /// Rich SFX voice with ADSR and optional filter, scaled by sfx_volume.
    fn sfx_voice(
        &self,
        freq: f32,
        wave: OscillatorType,
        vol: f32,
        env: &Adsr,
    ) {
        let vol = vol * self.sfx_volume;
        if vol <= 0.0 { return; }
        let dest = self.dest();
        let t0 = self.now();
        self.layered_voice(freq, wave, vol, env, &dest, t0);
    }

    // ── Music ───────────────────────────────────────────────────────

    /// Called from animation loop — plays ambient music phrases.
    pub fn tick_music(&mut self) {
        let now = self.ctx.current_time();
        if now < self.music_next || self.music_mood == MusicMood::Silent {
            return;
        }
        match self.music_mood {
            MusicMood::Explore => {
                self.pad(130.81, 4.0, 0.03); // C3
                self.pad(196.00, 4.0, 0.025); // G3
                self.pad(261.63, 3.5, 0.02); // C4
                self.music_next = now + 3.5;
            }
            MusicMood::Combat => {
                self.pad(146.83, 2.5, 0.04); // D3
                self.pad(174.61, 2.5, 0.035); // F3
                self.pad(220.00, 2.0, 0.03); // A3
                self.pulse(73.42, 0.15, 0.05); // D2 pulse
                self.music_next = now + 2.0;
            }
            MusicMood::Boss => {
                self.pad(98.00, 3.0, 0.05); // G2
                self.pad(116.54, 3.0, 0.04); // Bb2
                self.pad(146.83, 2.5, 0.035); // D3
                self.pulse(49.00, 0.2, 0.06); // G1 pulse
                self.music_next = now + 1.8;
            }
            MusicMood::Silent => {}
        }
    }

    /// Soft ambient pad tone with fade-in/out (layered for warmth).
    fn pad(&self, freq: f32, duration: f64, volume: f32) {
        let volume = volume * self.music_volume;
        if volume <= 0.0 { return; }

        // Two slightly detuned sine oscillators for chorus effect
        for &detune in &[-5.0_f32, 5.0] {
            let osc = match self.osc() { Some(o) => o, None => return };
            let g = match self.gain() { Some(g) => g, None => return };
            osc.set_type(OscillatorType::Sine);
            osc.frequency().set_value(freq);
            osc.detune().set_value(detune);
            let now = self.now();
            g.gain().set_value(0.0);
            let _ = g.gain().linear_ramp_to_value_at_time(volume * 0.6, now + 0.3);
            let _ = g.gain().linear_ramp_to_value_at_time(volume * 0.6, now + duration - 0.5);
            let _ = g.gain().linear_ramp_to_value_at_time(0.0, now + duration);
            let dest = self.dest();
            let _ = osc.connect_with_audio_node(&g);
            let _ = g.connect_with_audio_node(&dest);
            let _ = osc.start();
            let _ = osc.stop_with_when(now + duration);
        }
    }

    /// Low rhythmic pulse.
    fn pulse(&self, freq: f32, duration: f64, volume: f32) {
        let volume = volume * self.music_volume;
        if volume <= 0.0 { return; }
        let osc = match self.osc() { Some(o) => o, None => return };
        let g = match self.gain() { Some(g) => g, None => return };
        osc.set_type(OscillatorType::Triangle);
        osc.frequency().set_value(freq);
        let now = self.now();
        g.gain().set_value(volume);
        let _ = g.gain().linear_ramp_to_value_at_time(0.0, now + duration);
        let dest = self.dest();
        let _ = osc.connect_with_audio_node(&g);
        let _ = g.connect_with_audio_node(&dest);
        let _ = osc.start();
        let _ = osc.stop_with_when(now + duration);
    }

    // ── Combat Sounds ───────────────────────────────────────────────

    /// Player hits enemy correctly — layered square + triangle with ADSR.
    pub fn play_hit(&self) {
        self.sfx_voice(440.0, OscillatorType::Square, 0.12, &Adsr::quick());
        self.sfx_voice(660.0, OscillatorType::Triangle, 0.08, &Adsr::quick());
    }

    /// Player misses (wrong pinyin) — descending whoosh.
    pub fn play_miss(&self) {
        self.sweep(400.0, 100.0, 0.2, OscillatorType::Sawtooth, 0.10);
        self.filtered_tone(
            120.0, OscillatorType::Sawtooth, 0.08 * self.sfx_volume,
            &Adsr::medium(), 600.0, BiquadFilterType::Lowpass,
        );
    }

    /// Enemy killed — descending tone with filtered fade.
    pub fn play_kill(&self) {
        self.sfx_voice(330.0, OscillatorType::Square, 0.10, &Adsr::quick());
        self.sfx_voice(440.0, OscillatorType::Square, 0.10, &Adsr::quick());
        self.sfx_voice(660.0, OscillatorType::Triangle, 0.08, &Adsr::medium());
    }

    /// Enemy death — descending tone with long fade.
    pub fn play_enemy_death(&self) {
        self.sweep(500.0, 120.0, 0.4, OscillatorType::Sawtooth, 0.10);
        self.filtered_tone(
            100.0, OscillatorType::Triangle, 0.06 * self.sfx_volume,
            &Adsr::soft(), 400.0, BiquadFilterType::Lowpass,
        );
    }

    /// Critical hit — sharp attack + reverb-like echo.
    pub fn play_critical_hit(&self) {
        self.sfx_voice(880.0, OscillatorType::Square, 0.14, &Adsr::percussive());
        self.sfx_voice(660.0, OscillatorType::Triangle, 0.10, &Adsr::percussive());
        // Delayed quieter repeat for reverb feel
        self.tone_at(880.0, 0.08, 0.06, OscillatorType::Square, 0.12);
        self.tone_at(660.0, 0.06, 0.04, OscillatorType::Triangle, 0.14);
    }

    /// Projectile launched — quick ascending blip.
    pub fn play_projectile_launch(&self) {
        self.sweep(300.0, 900.0, 0.1, OscillatorType::Triangle, 0.10);
    }

    /// Projectile impact — percussive thud with short decay.
    pub fn play_projectile_impact(&self) {
        self.sfx_voice(100.0, OscillatorType::Triangle, 0.12, &Adsr::percussive());
        self.tone(60.0, 0.08, 0.10, OscillatorType::Sine);
    }

    /// Heal — warm ascending arpeggio (3 quick rising tones).
    pub fn play_heal(&self) {
        self.tone_at(523.0, 0.12, 0.10, OscillatorType::Sine, 0.0);   // C5
        self.tone_at(659.0, 0.12, 0.09, OscillatorType::Sine, 0.08);  // E5
        self.tone_at(784.0, 0.18, 0.08, OscillatorType::Sine, 0.16);  // G5
    }

    /// Shield block — metallic clang (square wave + high-frequency burst).
    pub fn play_shield_block(&self) {
        self.sfx_voice(800.0, OscillatorType::Square, 0.12, &Adsr::percussive());
        self.sfx_voice(1200.0, OscillatorType::Square, 0.06, &Adsr::percussive());
        self.tone(2400.0, 0.03, 0.05, OscillatorType::Sawtooth);
    }

    /// Status effect applied: burn — crackling high burst.
    pub fn play_status_burn(&self) {
        self.tone(900.0, 0.04, 0.08, OscillatorType::Sawtooth);
        self.tone_at(1100.0, 0.03, 0.06, OscillatorType::Sawtooth, 0.05);
        self.tone_at(800.0, 0.03, 0.05, OscillatorType::Sawtooth, 0.09);
    }

    /// Status effect: poison — bubbling low tone.
    pub fn play_status_poison(&self) {
        self.tone(120.0, 0.06, 0.08, OscillatorType::Sine);
        self.tone_at(140.0, 0.05, 0.07, OscillatorType::Sine, 0.07);
        self.tone_at(110.0, 0.05, 0.06, OscillatorType::Sine, 0.13);
    }

    /// Status effect: slow — descending drone.
    pub fn play_status_slow(&self) {
        self.sweep(300.0, 100.0, 0.3, OscillatorType::Triangle, 0.08);
    }

    /// Forge success — bright ascending chime with warmth.
    pub fn play_forge(&self) {
        self.sfx_voice(523.0, OscillatorType::Sine, 0.10, &Adsr::medium());
        self.tone_at(659.0, 0.12, 0.09, OscillatorType::Triangle, 0.10);
        self.tone_at(784.0, 0.18, 0.07, OscillatorType::Sine, 0.18);
    }

    /// Spell cast — rising tone sweep.
    pub fn play_spell(&self) {
        self.sweep(400.0, 1200.0, 0.15, OscillatorType::Sine, 0.10);
        self.tone_at(1100.0, 0.10, 0.06, OscillatorType::Triangle, 0.10);
    }

    /// Element-specific spell cast with per-element pitch character.
    pub fn play_spell_element(&self, element: &str) {
        match element {
            "fire" | "Fire" => {
                self.sweep(600.0, 1400.0, 0.15, OscillatorType::Sawtooth, 0.10);
                self.tone_at(1200.0, 0.08, 0.06, OscillatorType::Square, 0.10);
            }
            "water" | "Water" => {
                self.sweep(200.0, 500.0, 0.2, OscillatorType::Sine, 0.10);
                self.tone_at(400.0, 0.15, 0.06, OscillatorType::Triangle, 0.12);
            }
            "earth" | "Earth" => {
                self.sweep(80.0, 250.0, 0.2, OscillatorType::Triangle, 0.12);
                self.tone(60.0, 0.15, 0.08, OscillatorType::Sine);
            }
            "metal" | "Metal" => {
                self.sweep(800.0, 2000.0, 0.12, OscillatorType::Square, 0.09);
                self.tone_at(1800.0, 0.06, 0.05, OscillatorType::Sawtooth, 0.08);
            }
            "wood" | "Wood" => {
                self.sweep(300.0, 700.0, 0.18, OscillatorType::Triangle, 0.10);
                self.tone_at(600.0, 0.12, 0.06, OscillatorType::Sine, 0.10);
            }
            _ => self.play_spell(),
        }
    }

    /// Player takes damage — harsh filtered crunch.
    pub fn play_damage(&self) {
        self.sfx_voice(180.0, OscillatorType::Sawtooth, 0.12, &Adsr::percussive());
        self.filtered_tone(
            120.0, OscillatorType::Sawtooth, 0.10 * self.sfx_volume,
            &Adsr::medium(), 500.0, BiquadFilterType::Lowpass,
        );
    }

    /// Descend to next floor — ominous descending tones with filter.
    pub fn play_descend(&self) {
        self.tone_at(440.0, 0.15, 0.08, OscillatorType::Sine, 0.0);
        self.tone_at(330.0, 0.15, 0.08, OscillatorType::Sine, 0.12);
        self.tone_at(262.0, 0.25, 0.07, OscillatorType::Triangle, 0.24);
    }

    /// Player death — dramatic descending filtered tones.
    pub fn play_death(&self) {
        self.sfx_voice(300.0, OscillatorType::Sawtooth, 0.12, &Adsr::soft());
        self.tone_at(200.0, 0.3, 0.10, OscillatorType::Sawtooth, 0.15);
        self.tone_at(100.0, 0.5, 0.08, OscillatorType::Sawtooth, 0.35);
    }

    /// Shop purchase — bright two-note chime.
    pub fn play_buy(&self) {
        self.sfx_voice(800.0, OscillatorType::Triangle, 0.08, &Adsr::quick());
        self.tone_at(1000.0, 0.10, 0.07, OscillatorType::Triangle, 0.06);
    }

    /// Level up — triumphant ascending scale (C-E-G-C).
    pub fn play_level_up(&self) {
        self.tone_at(523.0, 0.10, 0.10, OscillatorType::Triangle, 0.0);   // C5
        self.tone_at(659.0, 0.10, 0.10, OscillatorType::Triangle, 0.10);  // E5
        self.tone_at(784.0, 0.10, 0.10, OscillatorType::Triangle, 0.20);  // G5
        self.tone_at(1047.0, 0.25, 0.12, OscillatorType::Sine, 0.30);     // C6
    }

    /// Victory — short fanfare.
    pub fn play_victory(&self) {
        self.tone_at(523.0, 0.12, 0.10, OscillatorType::Triangle, 0.0);
        self.tone_at(659.0, 0.12, 0.10, OscillatorType::Triangle, 0.10);
        self.tone_at(784.0, 0.12, 0.10, OscillatorType::Triangle, 0.20);
        self.tone_at(1047.0, 0.30, 0.12, OscillatorType::Sine, 0.30);
        self.tone_at(784.0, 0.15, 0.06, OscillatorType::Sine, 0.55);
        self.tone_at(1047.0, 0.40, 0.10, OscillatorType::Sine, 0.65);
    }

    /// Combat start — dramatic sting (quick minor chord).
    pub fn play_combat_start(&self) {
        self.sfx_voice(220.0, OscillatorType::Sawtooth, 0.10, &Adsr::quick());
        self.sfx_voice(261.63, OscillatorType::Sawtooth, 0.08, &Adsr::quick());
        self.sfx_voice(329.63, OscillatorType::Sawtooth, 0.08, &Adsr::quick());
    }

    /// Boss encounter — deeper, more ominous version of combat start.
    pub fn play_boss_encounter(&self) {
        self.sfx_voice(110.0, OscillatorType::Sawtooth, 0.12, &Adsr::medium());
        self.sfx_voice(130.81, OscillatorType::Sawtooth, 0.10, &Adsr::medium());
        self.sfx_voice(164.81, OscillatorType::Sawtooth, 0.10, &Adsr::medium());
        self.tone(55.0, 0.4, 0.08, OscillatorType::Sine);
    }

    // ── Movement / Environmental ────────────────────────────────────

    /// Footstep — subtle tap (pitch varies with `terrain_hint`).
    /// 0 = normal floor, 1 = stone, 2 = grass/soft, 3 = metal
    pub fn play_step(&self) {
        self.play_step_terrain(0);
    }

    pub fn play_step_terrain(&self, terrain_hint: u8) {
        let (freq, vol) = match terrain_hint {
            1 => (100.0_f32, 0.04_f32), // stone
            2 => (60.0, 0.03),          // grass/soft
            3 => (200.0, 0.05),         // metal
            _ => (80.0, 0.04),          // default
        };
        let vol = vol * self.sfx_volume;
        if vol <= 0.0 { return; }
        let osc = match self.osc() { Some(o) => o, None => return };
        let g = match self.gain() { Some(g) => g, None => return };
        osc.set_type(OscillatorType::Triangle);
        osc.frequency().set_value(freq);
        let t0 = self.now();
        g.gain().set_value(vol);
        let _ = g.gain().linear_ramp_to_value_at_time(0.0, t0 + 0.04);
        let dest = self.dest();
        let _ = osc.connect_with_audio_node(&g);
        let _ = g.connect_with_audio_node(&dest);
        let _ = osc.start();
        let _ = osc.stop_with_when(t0 + 0.05);
    }

    /// Water tile — soft splash.
    pub fn play_water_splash(&self) {
        self.filtered_tone(
            200.0, OscillatorType::Sine, 0.05 * self.sfx_volume,
            &Adsr::soft(), 800.0, BiquadFilterType::Lowpass,
        );
        self.tone(400.0, 0.06, 0.03, OscillatorType::Sine);
    }

    /// Lava proximity — low rumble.
    pub fn play_lava_rumble(&self) {
        self.filtered_tone(
            40.0, OscillatorType::Sawtooth, 0.04 * self.sfx_volume,
            &Adsr::pad_env(), 200.0, BiquadFilterType::Lowpass,
        );
    }

    /// Treasure/pickup — bright sparkle chime.
    pub fn play_treasure(&self) {
        self.tone_at(1200.0, 0.06, 0.08, OscillatorType::Triangle, 0.0);
        self.tone_at(1600.0, 0.06, 0.07, OscillatorType::Triangle, 0.05);
        self.tone_at(2000.0, 0.10, 0.06, OscillatorType::Sine, 0.10);
    }

    /// Digging through stone — gritty crunch.
    pub fn play_dig(&self) {
        self.sfx_voice(120.0, OscillatorType::Sawtooth, 0.08, &Adsr::percussive());
        self.tone(92.0, 0.10, 0.07, OscillatorType::Triangle);
        self.tone_at(180.0, 0.08, 0.05, OscillatorType::Square, 0.05);
    }

    /// Crate splashing into place as a bridge.
    pub fn play_bridge(&self) {
        self.sfx_voice(220.0, OscillatorType::Triangle, 0.07, &Adsr::medium());
        self.tone_at(330.0, 0.10, 0.06, OscillatorType::Sine, 0.06);
        self.filtered_tone(
            180.0, OscillatorType::Sawtooth, 0.04 * self.sfx_volume,
            &Adsr::soft(), 600.0, BiquadFilterType::Lowpass,
        );
    }

    /// Streak ding — bright ascending chime.
    pub fn play_streak_ding(&self) {
        self.tone_at(660.0, 0.06, 0.08, OscillatorType::Sine, 0.0);
        self.tone_at(880.0, 0.08, 0.07, OscillatorType::Sine, 0.05);
        self.tone_at(1100.0, 0.12, 0.06, OscillatorType::Triangle, 0.10);
    }

    // ── UI Feedback ─────────────────────────────────────────────────

    /// Menu open/close — soft click.
    pub fn play_menu_click(&self) {
        self.tone(600.0, 0.02, 0.06, OscillatorType::Triangle);
    }

    /// Console toggle — mechanical keyboard clack.
    pub fn play_console_toggle(&self) {
        self.sfx_voice(1000.0, OscillatorType::Square, 0.08, &Adsr::percussive());
        self.tone(500.0, 0.02, 0.05, OscillatorType::Triangle);
    }

    /// Typing correct — positive ding.
    pub fn play_typing_correct(&self) {
        self.tone(1200.0, 0.06, 0.06, OscillatorType::Sine);
    }

    /// Typing incorrect — error buzz (low square wave).
    pub fn play_typing_error(&self) {
        self.tone(120.0, 0.12, 0.08, OscillatorType::Square);
    }

    // ── Space Combat Sounds ─────────────────────────────────────────

    /// Laser weapon fire — quick ascending zap.
    pub fn play_laser_fire(&self) {
        self.sweep(200.0, 1800.0, 0.08, OscillatorType::Sawtooth, 0.09);
        self.tone(1200.0, 0.04, 0.06, OscillatorType::Sine);
    }

    /// Missile launch — rumbling whoosh ascending.
    pub fn play_missile_launch(&self) {
        self.sweep(80.0, 400.0, 0.25, OscillatorType::Triangle, 0.08);
        self.tone_at(200.0, 0.15, 0.06, OscillatorType::Square, 0.1);
    }

    /// Ion cannon — electric crackling discharge.
    pub fn play_ion_cannon(&self) {
        self.sweep(600.0, 200.0, 0.15, OscillatorType::Square, 0.10);
        self.sweep(1200.0, 300.0, 0.12, OscillatorType::Sawtooth, 0.07);
        self.tone_at(150.0, 0.1, 0.08, OscillatorType::Square, 0.08);
    }

    /// Broadside — multiple thuds in rapid succession.
    pub fn play_broadside(&self) {
        self.tone_at(100.0, 0.08, 0.10, OscillatorType::Triangle, 0.0);
        self.tone_at(120.0, 0.08, 0.09, OscillatorType::Triangle, 0.06);
        self.tone_at(90.0, 0.08, 0.10, OscillatorType::Triangle, 0.12);
        self.tone_at(110.0, 0.08, 0.08, OscillatorType::Triangle, 0.18);
    }

    /// Shield recharge — ascending shimmer.
    pub fn play_shield_recharge(&self) {
        self.sweep(400.0, 1200.0, 0.3, OscillatorType::Sine, 0.06);
        self.tone_at(800.0, 0.2, 0.04, OscillatorType::Triangle, 0.15);
    }

    /// Subsystem damaged — metallic crunch.
    #[allow(dead_code)]
    pub fn play_subsystem_damage(&self) {
        self.sfx_voice(150.0, OscillatorType::Square, 0.12, &Adsr::percussive());
        self.tone(80.0, 0.15, 0.10, OscillatorType::Sawtooth);
    }

    /// Subsystem destroyed — alarm-like descending wail.
    pub fn play_subsystem_destroyed(&self) {
        self.sweep(800.0, 200.0, 0.3, OscillatorType::Sawtooth, 0.10);
        self.tone_at(150.0, 0.2, 0.08, OscillatorType::Square, 0.15);
    }

    /// Evasive maneuver — quick swoosh.
    pub fn play_evasion(&self) {
        self.sweep(600.0, 200.0, 0.1, OscillatorType::Triangle, 0.07);
        self.sweep(200.0, 600.0, 0.1, OscillatorType::Sine, 0.05);
    }

    /// Enemy ship weapon fire — slightly different from player.
    pub fn play_enemy_weapon_fire(&self) {
        self.sweep(300.0, 100.0, 0.12, OscillatorType::Square, 0.08);
        self.tone(80.0, 0.1, 0.07, OscillatorType::Triangle);
    }

    /// Boarding attempt — tense ascending tones.
    pub fn play_boarding(&self) {
        self.tone_at(200.0, 0.15, 0.08, OscillatorType::Triangle, 0.0);
        self.tone_at(300.0, 0.15, 0.07, OscillatorType::Triangle, 0.1);
        self.tone_at(400.0, 0.15, 0.06, OscillatorType::Triangle, 0.2);
    }

    /// Missile miss — descending whiff.
    pub fn play_missile_miss(&self) {
        self.sweep(400.0, 100.0, 0.15, OscillatorType::Triangle, 0.06);
    }

    // ── Arena Terrain Sounds ────────────────────────────────────────

    /// Gravity well pull — deep resonant hum.
    pub fn play_gravity_pull(&self) {
        self.sweep(200.0, 60.0, 0.3, OscillatorType::Sine, 0.08);
        self.tone(40.0, 0.2, 0.06, OscillatorType::Triangle);
    }

    /// Steam vent activation — hissing rush.
    pub fn play_steam_vent(&self) {
        self.tone(2000.0, 0.15, 0.04, OscillatorType::Square);
        self.tone(2500.0, 0.12, 0.03, OscillatorType::Square);
        self.sweep(3000.0, 1500.0, 0.2, OscillatorType::Square, 0.03);
    }

    /// Oil/lubricant ignition — whoompf.
    pub fn play_oil_ignition(&self) {
        self.sweep(100.0, 400.0, 0.12, OscillatorType::Triangle, 0.10);
        self.tone_at(300.0, 0.1, 0.08, OscillatorType::Sawtooth, 0.05);
    }

    /// Crate sliding/chain push — scraping thud.
    pub fn play_crate_push(&self) {
        self.tone(100.0, 0.1, 0.08, OscillatorType::Triangle);
        self.tone_at(80.0, 0.08, 0.10, OscillatorType::Square, 0.05);
    }

    /// Crate crushing a unit — heavy impact.
    pub fn play_crate_crush(&self) {
        self.sfx_voice(80.0, OscillatorType::Square, 0.14, &Adsr::percussive());
        self.tone(50.0, 0.15, 0.12, OscillatorType::Triangle);
        self.tone_at(120.0, 0.08, 0.08, OscillatorType::Sawtooth, 0.05);
    }

    /// Conveyor engagement — mechanical whir.
    pub fn play_conveyor(&self) {
        self.sweep(150.0, 300.0, 0.15, OscillatorType::Triangle, 0.05);
        self.tone(200.0, 0.1, 0.04, OscillatorType::Square);
    }

    /// Explosion chain reaction — escalating booms.
    pub fn play_chain_explosion(&self) {
        self.tone_at(80.0, 0.12, 0.12, OscillatorType::Triangle, 0.0);
        self.tone_at(100.0, 0.14, 0.14, OscillatorType::Triangle, 0.08);
        self.tone_at(60.0, 0.18, 0.16, OscillatorType::Sawtooth, 0.16);
    }

    /// Turn advance — subtle tick.
    pub fn play_turn_tick(&self) {
        self.tone(400.0, 0.015, 0.03, OscillatorType::Triangle);
    }

    // ── Chinese Tone Contours ───────────────────────────────────────

    /// Play a Chinese tone contour for listening mode.
    /// tone_num: 1-4 (Chinese tones), base_freq ~300 Hz for a natural voice range.
    pub fn play_chinese_tone(&self, tone_num: u8) {
        if self.sfx_volume <= 0.0 {
            return;
        }
        // Use two detuned oscillators for warmth
        for &detune in &[-6.0_f32, 6.0] {
            let osc = match self.osc() { Some(o) => o, None => return };
            let g = match self.gain() { Some(g) => g, None => return };
            osc.set_type(OscillatorType::Sine);
            osc.detune().set_value(detune);
            let now = self.now();
            let dur = 0.5;
            let volume = 0.08 * self.sfx_volume;
            g.gain().set_value(volume);
            let _ = g.gain().linear_ramp_to_value_at_time(volume, now + dur - 0.1);
            let _ = g.gain().linear_ramp_to_value_at_time(0.0, now + dur);

            match tone_num {
                1 => {
                    osc.frequency().set_value(350.0);
                }
                2 => {
                    osc.frequency().set_value(250.0);
                    let _ = osc.frequency().linear_ramp_to_value_at_time(380.0, now + dur);
                }
                3 => {
                    osc.frequency().set_value(280.0);
                    let _ = osc.frequency().linear_ramp_to_value_at_time(220.0, now + dur * 0.5);
                    let _ = osc.frequency().linear_ramp_to_value_at_time(300.0, now + dur);
                }
                4 => {
                    osc.frequency().set_value(380.0);
                    let _ = osc.frequency().linear_ramp_to_value_at_time(200.0, now + dur);
                }
                _ => {
                    osc.frequency().set_value(280.0);
                }
            }

            let dest = self.dest();
            let _ = osc.connect_with_audio_node(&g);
            let _ = g.connect_with_audio_node(&dest);
            let _ = osc.start();
            let _ = osc.stop_with_when(now + dur);
        }
    }

    // ── Additional SFX ──────────────────────────────────────────────

    /// Space jump — whoosh sweep up then down.
    pub fn sfx_jump(&self) {
        if self.sfx_volume < 0.01 { return; }
        let now = self.now();
        let osc = match self.osc() { Some(o) => o, None => return };
        let g = match self.gain() { Some(g) => g, None => return };
        osc.set_type(OscillatorType::Sine);
        osc.frequency().set_value(100.0);
        let _ = osc.frequency().exponential_ramp_to_value_at_time(2000.0, now + 0.15);
        let _ = osc.frequency().exponential_ramp_to_value_at_time(200.0, now + 0.4);
        g.gain().set_value(0.0);
        let _ = g.gain().linear_ramp_to_value_at_time(0.2 * self.sfx_volume, now + 0.05);
        let _ = g.gain().linear_ramp_to_value_at_time(0.0, now + 0.4);
        let _ = osc.connect_with_audio_node(&g);
        let _ = g.connect_with_audio_node(&self.dest());
        let _ = osc.start();
        let _ = osc.stop_with_when(now + 0.45);
    }

    /// Door open — short mechanical click-swoosh.
    pub fn sfx_door(&self) {
        if self.sfx_volume < 0.01 { return; }
        let now = self.now();
        let osc = match self.osc() { Some(o) => o, None => return };
        let g = match self.gain() { Some(g) => g, None => return };
        osc.set_type(OscillatorType::Square);
        osc.frequency().set_value(800.0);
        let _ = osc.frequency().exponential_ramp_to_value_at_time(200.0, now + 0.08);
        g.gain().set_value(0.0);
        let _ = g.gain().linear_ramp_to_value_at_time(0.1 * self.sfx_volume, now + 0.005);
        let _ = g.gain().linear_ramp_to_value_at_time(0.0, now + 0.1);
        let _ = osc.connect_with_audio_node(&g);
        let _ = g.connect_with_audio_node(&self.dest());
        let _ = osc.start();
        let _ = osc.stop_with_when(now + 0.12);
    }

    /// Correct answer — pleasant ding.
    pub fn sfx_correct(&self) {
        if self.sfx_volume < 0.01 { return; }
        let now = self.now();
        let osc = match self.osc() { Some(o) => o, None => return };
        let g = match self.gain() { Some(g) => g, None => return };
        osc.set_type(OscillatorType::Sine);
        osc.frequency().set_value(880.0);
        g.gain().set_value(0.0);
        let _ = g.gain().linear_ramp_to_value_at_time(0.15 * self.sfx_volume, now + 0.01);
        let _ = g.gain().linear_ramp_to_value_at_time(0.0, now + 0.25);
        let _ = osc.connect_with_audio_node(&g);
        let _ = g.connect_with_audio_node(&self.dest());
        let _ = osc.start();
        let _ = osc.stop_with_when(now + 0.3);
    }

    /// Wrong answer — low buzz.
    pub fn sfx_wrong(&self) {
        if self.sfx_volume < 0.01 { return; }
        let now = self.now();
        let osc = match self.osc() { Some(o) => o, None => return };
        let g = match self.gain() { Some(g) => g, None => return };
        osc.set_type(OscillatorType::Sawtooth);
        osc.frequency().set_value(100.0);
        g.gain().set_value(0.0);
        let _ = g.gain().linear_ramp_to_value_at_time(0.2 * self.sfx_volume, now + 0.01);
        let _ = g.gain().linear_ramp_to_value_at_time(0.0, now + 0.2);
        let _ = osc.connect_with_audio_node(&g);
        let _ = g.connect_with_audio_node(&self.dest());
        let _ = osc.start();
        let _ = osc.stop_with_when(now + 0.25);
    }
}
