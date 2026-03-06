//! Procedural audio using Web Audio API.
//! All sounds are synthesized — no audio files needed.

use wasm_bindgen::JsCast;
use web_sys::{AudioContext, GainNode, OscillatorNode, OscillatorType};

/// Current music mood.
#[derive(Clone, Copy, PartialEq)]
pub enum MusicMood {
    Explore,
    Combat,
    Boss,
    Silent,
}

pub struct Audio {
    ctx: AudioContext,
    /// Time when next ambient phrase should play.
    music_next: f64,
    /// Current music mood.
    music_mood: MusicMood,
}

impl Audio {
    pub fn new() -> Option<Self> {
        AudioContext::new().ok().map(|ctx| Self {
            music_next: 0.0,
            music_mood: MusicMood::Explore,
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
            self.music_next = 0.0; // trigger immediate phrase
        }
    }

    /// Called from animation loop — plays ambient music phrases.
    pub fn tick_music(&mut self) {
        let now = self.ctx.current_time();
        if now < self.music_next || self.music_mood == MusicMood::Silent {
            return;
        }
        match self.music_mood {
            MusicMood::Explore => {
                // Calm ambient drone: two detuned sine pads
                self.pad(130.81, 4.0, 0.03); // C3
                self.pad(196.00, 4.0, 0.025); // G3
                self.pad(261.63, 3.5, 0.02); // C4
                self.music_next = now + 3.5;
            }
            MusicMood::Combat => {
                // Tense: minor chord pads + low pulse
                self.pad(146.83, 2.5, 0.04); // D3
                self.pad(174.61, 2.5, 0.035); // F3
                self.pad(220.00, 2.0, 0.03); // A3
                self.pulse(73.42, 0.15, 0.05); // D2 pulse
                self.music_next = now + 2.0;
            }
            MusicMood::Boss => {
                // Dramatic: low drone + dissonant overtone
                self.pad(98.00, 3.0, 0.05); // G2
                self.pad(116.54, 3.0, 0.04); // Bb2
                self.pad(146.83, 2.5, 0.035); // D3
                self.pulse(49.00, 0.2, 0.06); // G1 pulse
                self.music_next = now + 1.8;
            }
            MusicMood::Silent => {}
        }
    }

    /// Soft ambient pad tone with fade-in and fade-out.
    fn pad(&self, freq: f32, duration: f64, volume: f32) {
        let osc = match self.ctx.create_oscillator() {
            Ok(o) => o,
            Err(_) => return,
        };
        let gain = match self.ctx.create_gain() {
            Ok(g) => g,
            Err(_) => return,
        };
        osc.set_type(OscillatorType::Sine);
        osc.frequency().set_value(freq);
        let now = self.ctx.current_time();
        gain.gain().set_value(0.0);
        gain.gain().linear_ramp_to_value_at_time(volume, now + 0.3).ok();
        gain.gain().linear_ramp_to_value_at_time(volume, now + duration - 0.5).ok();
        gain.gain().linear_ramp_to_value_at_time(0.0, now + duration).ok();
        let _ = osc.connect_with_audio_node(&gain);
        let _ = gain.connect_with_audio_node(&self.ctx.destination());
        let _ = osc.start();
        let _ = osc.stop_with_when(now + duration);
    }

    /// Low rhythmic pulse.
    fn pulse(&self, freq: f32, duration: f64, volume: f32) {
        let osc = match self.ctx.create_oscillator() {
            Ok(o) => o,
            Err(_) => return,
        };
        let gain = match self.ctx.create_gain() {
            Ok(g) => g,
            Err(_) => return,
        };
        osc.set_type(OscillatorType::Triangle);
        osc.frequency().set_value(freq);
        let now = self.ctx.current_time();
        gain.gain().set_value(volume);
        gain.gain().linear_ramp_to_value_at_time(0.0, now + duration).ok();
        let _ = osc.connect_with_audio_node(&gain);
        let _ = gain.connect_with_audio_node(&self.ctx.destination());
        let _ = osc.start();
        let _ = osc.stop_with_when(now + duration);
    }

    fn tone(&self, freq: f32, duration: f64, volume: f32, wave: OscillatorType) {
        let osc: OscillatorNode = match self.ctx.create_oscillator() {
            Ok(o) => o,
            Err(_) => return,
        };
        let gain: GainNode = match self.ctx.create_gain() {
            Ok(g) => g,
            Err(_) => return,
        };
        osc.set_type(wave);
        osc.frequency().set_value(freq);
        gain.gain().set_value(volume);

        let now = self.ctx.current_time();
        // Fade out
        gain.gain()
            .linear_ramp_to_value_at_time(0.0, now + duration)
            .ok();

        let _ = osc.connect_with_audio_node(&gain);
        let _ = gain.connect_with_audio_node(&self.ctx.destination());
        let _ = osc.start();
        let _ = osc.stop_with_when(now + duration);
    }

    /// Player hits enemy correctly
    pub fn play_hit(&self) {
        self.tone(440.0, 0.1, 0.15, OscillatorType::Square);
        self.tone(660.0, 0.08, 0.1, OscillatorType::Square);
    }

    /// Player misses (wrong pinyin)
    pub fn play_miss(&self) {
        self.tone(150.0, 0.2, 0.15, OscillatorType::Sawtooth);
        self.tone(100.0, 0.3, 0.12, OscillatorType::Sawtooth);
    }

    /// Enemy killed
    pub fn play_kill(&self) {
        self.tone(330.0, 0.08, 0.12, OscillatorType::Square);
        self.tone(440.0, 0.08, 0.12, OscillatorType::Square);
        self.tone(660.0, 0.15, 0.1, OscillatorType::Square);
    }

    /// Forge success
    pub fn play_forge(&self) {
        self.tone(523.0, 0.12, 0.12, OscillatorType::Sine);
        self.tone(659.0, 0.12, 0.1, OscillatorType::Sine);
        self.tone(784.0, 0.2, 0.08, OscillatorType::Sine);
    }

    /// Forge failure
    pub fn play_forge_fail(&self) {
        self.tone(200.0, 0.15, 0.12, OscillatorType::Triangle);
        self.tone(160.0, 0.2, 0.1, OscillatorType::Triangle);
    }

    /// Spell cast
    pub fn play_spell(&self) {
        self.tone(880.0, 0.06, 0.1, OscillatorType::Sine);
        self.tone(1100.0, 0.1, 0.08, OscillatorType::Sine);
        self.tone(1320.0, 0.15, 0.06, OscillatorType::Sine);
    }

    /// Player takes damage
    pub fn play_damage(&self) {
        self.tone(180.0, 0.12, 0.15, OscillatorType::Sawtooth);
        self.tone(120.0, 0.15, 0.12, OscillatorType::Sawtooth);
    }

    /// Descend to next floor
    pub fn play_descend(&self) {
        self.tone(440.0, 0.15, 0.1, OscillatorType::Sine);
        self.tone(330.0, 0.15, 0.1, OscillatorType::Sine);
        self.tone(262.0, 0.25, 0.08, OscillatorType::Sine);
    }

    /// Player death
    pub fn play_death(&self) {
        self.tone(300.0, 0.2, 0.15, OscillatorType::Sawtooth);
        self.tone(200.0, 0.3, 0.12, OscillatorType::Sawtooth);
        self.tone(100.0, 0.5, 0.1, OscillatorType::Sawtooth);
    }

    /// Shop purchase
    pub fn play_buy(&self) {
        self.tone(800.0, 0.05, 0.1, OscillatorType::Square);
        self.tone(1000.0, 0.1, 0.08, OscillatorType::Square);
    }

    /// Footstep (quiet)
    pub fn play_step(&self) {
        self.tone(80.0, 0.04, 0.04, OscillatorType::Triangle);
    }
}
