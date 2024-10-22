//! Single synthesis voice with engine dispatch and parameter processing.

// Based on MIT-licensed code (c) 2016 by Emilie Gillet (emilie.o.gillet@gmail.com)

use core::alloc::GlobalAlloc;

#[allow(unused_imports)]
use num_traits::float::Float;

use super::engine::additive_engine::AdditiveEngine;
use super::engine::bass_drum_engine::BassDrumEngine;
use super::engine::chord_engine::ChordEngine;
use super::engine::fm_engine::FmEngine;
use super::engine::grain_engine::GrainEngine;
use super::engine::hihat_engine::HihatEngine;
use super::engine::modal_engine::ModalEngine;
use super::engine::noise_engine::NoiseEngine;
use super::engine::particle_engine::ParticleEngine;
use super::engine::snare_drum_engine::SnareDrumEngine;
use super::engine::speech_engine::SpeechEngine;
use super::engine::string_engine::StringEngine;
use super::engine::swarm_engine::SwarmEngine;
use super::engine::virtual_analog_engine::VirtualAnalogEngine;
use super::engine::waveshaping_engine::WaveshapingEngine;
use super::engine::wavetable_engine::WavetableEngine;
use super::engine::{note_to_frequency, Engine, EngineParameters, TriggerState};
use super::engine2::chiptune_engine::{self, ChiptuneEngine};
use super::engine2::phase_distortion_engine::PhaseDistortionEngine;
use super::engine2::six_op_engine::SixOpEngine;
use super::engine2::string_machine_engine::StringMachineEngine;
use super::engine2::virtual_analog_vcf_engine::VirtualAnalogVcfEngine;
use super::engine2::wave_terrain_engine::WaveTerrainEngine;
use super::envelope::{DecayEnvelope, LpgEnvelope};
use super::fx::low_pass_gate::LowPassGate;
use super::physical_modelling::delay_line::DelayLine;
use crate::dsp::resources::sysex::{SYX_BANK_0, SYX_BANK_1, SYX_BANK_2};
use crate::dsp::resources::waves::WAV_INTEGRATED_WAVES;
use crate::dsp::{allocate_buffer, SAMPLE_RATE};
use crate::stmlib::dsp::clip_16;
use crate::stmlib::dsp::hysteresis_quantizer::HysteresisQuantizer2;
use crate::stmlib::dsp::limiter::Limiter;
use crate::stmlib::dsp::units::semitones_to_ratio;

const MAX_TRIGGER_DELAY: usize = 8;
pub const NUM_ENGINES: usize = 24;

/// Patch parameters.
#[derive(Debug, Clone)]
pub struct Patch {
    /// Note number in the range from `-119.0` to `120.0`. Default is `48.0`.
    pub note: f32,

    /// HARMONICS parameter in the range from `0.0` to `1.0`. Default is `0.5`.
    pub harmonics: f32,

    /// TIMBRE parameter in the range from `0.0` to `1.0`. Default is `0.5`.
    pub timbre: f32,

    /// MORPH parameter in the range from `0.0` to `1.0`. Default is `0.5`.
    pub morph: f32,

    /// Frequency modulation amount in the range from `-1.0` to `1.0`. Default is `0.0`.
    pub frequency_modulation_amount: f32,

    /// TIMBRE modulation amount in the range from `-1.0` to `1.0`. Default is `0.0`.
    pub timbre_modulation_amount: f32,

    /// MORPH modulation amount in the range from `-1.0` to `1.0`. Default is `0.0`.
    pub morph_modulation_amount: f32,

    /// Engine selection in the range from `0` to `23`. Default is `0`.
    pub engine: usize,

    /// Envelope decay in the range from `0.0` to `1.0`. Default is `0.5`.
    pub decay: f32,

    /// Low-pass gate color in the range from `0.0` to `1.0`. Default is `0.5`.
    pub lpg_colour: f32,
}

impl Default for Patch {
    fn default() -> Self {
        Self {
            note: 48.0,
            harmonics: 0.5,
            timbre: 0.5,
            morph: 0.5,
            frequency_modulation_amount: 0.0,
            timbre_modulation_amount: 0.0,
            morph_modulation_amount: 0.0,
            engine: 0,
            decay: 0.5,
            lpg_colour: 0.5,
        }
    }
}

/// Modulation parameters.
#[derive(Debug, Default, Clone)]
pub struct Modulations {
    /// Engine select modulation in the range from `-1.0` to `1.0`. Default is `0.0`.
    pub engine: f32,

    /// Note number modulation in the range from `-119.0` to `120.0`. Default is `0.0`.
    pub note: f32,

    /// Frequency modulation in the range from `-1.0` to `1.0`. Default is `0.0`.
    pub frequency: f32,

    /// HARMONICS modulation in the range from `-1.0` to `1.0`. Default is `0.0`.
    pub harmonics: f32,

    /// TIMBRE modulation in the range from `-1.0` to `1.0`. Default is `0.0`.
    pub timbre: f32,

    /// MORPH modulation in the range from `-1.0` to `1.0`. Default is `0.0`.
    pub morph: f32,

    /// Trigger signal in the range from `0.0` to `1.0`. Default is `0.0`.
    pub trigger: f32,

    /// Level modulation in the range from `0.0` to `1.0`. Default is `0.0`.
    pub level: f32,

    /// Flag if frequency modulation is applied. Default is `false`.
    pub frequency_patched: bool,

    /// Flag if timbre modulation is applied. Default is `false`.
    pub timbre_patched: bool,

    /// Flag if morph modulation is applied. Default is `false`.
    pub morph_patched: bool,

    /// Flag if trigger signal is used. Default is `false`.
    pub trigger_patched: bool,

    /// Flag if level modulation is used. Default is `false`.
    pub level_patched: bool,
}

/// Resources used by some of the engines. The provided data is loaded when an engine
/// is selected. Call `Voice::reload_resources` to force an update without changing the engine.
#[derive(Debug, Clone)]
pub struct Resources<'a> {
    /// Sysex bank for six op engine 2. Default is integrated `SYX_BANK_0`.
    pub syx_bank_a: &'a [u8; 4096],

    /// Sysex bank for six op engine 3. Default is integrated `SYX_BANK_1`.
    pub syx_bank_b: &'a [u8; 4096],

    /// Sysex bank for six op engine 4. Default is integrated `SYX_BANK_2`.
    pub syx_bank_c: &'a [u8; 4096],

    /// User terrain for the wave terrain engine. Default is `None`.
    pub user_wave_terrain: Option<&'a [u8; 4096]>,

    /// Wavetables for the wavetable engine. Default is integrated waves.
    pub wavetables: &'a [i16; 25344],
}

impl<'a> Default for Resources<'a> {
    fn default() -> Self {
        Self {
            syx_bank_a: &SYX_BANK_0,
            syx_bank_b: &SYX_BANK_1,
            syx_bank_c: &SYX_BANK_2,
            user_wave_terrain: None,
            wavetables: &WAV_INTEGRATED_WAVES,
        }
    }
}

#[derive(Debug)]
pub struct Voice<'a> {
    pub additive_engine: AdditiveEngine,
    pub bass_drum_engine: BassDrumEngine,
    pub chiptune_engine: ChiptuneEngine,
    pub chord_engine: ChordEngine<'a>,
    pub fm_engine: FmEngine,
    pub grain_engine: GrainEngine,
    pub hihat_engine: HihatEngine<'a>,
    pub modal_engine: ModalEngine<'a>,
    pub noise_engine: NoiseEngine<'a>,
    pub particle_engine: ParticleEngine<'a>,
    pub phase_distortion_engine: PhaseDistortionEngine<'a>,
    pub six_op_engine: SixOpEngine<'a>,
    pub snare_drum_engine: SnareDrumEngine,
    pub speech_engine: SpeechEngine<'a>,
    pub string_engine: StringEngine<'a>,
    pub string_machine_engine: StringMachineEngine,
    pub swarm_engine: SwarmEngine,
    pub virtual_analog_engine: VirtualAnalogEngine<'a>,
    pub virtual_analog_vcf_engine: VirtualAnalogVcfEngine,
    pub waveshaping_engine: WaveshapingEngine,
    pub wavetable_engine: WavetableEngine<'a>,
    pub waveterrain_engine: WaveTerrainEngine<'a>,

    pub resources: Resources<'a>,

    engine_quantizer: HysteresisQuantizer2,

    reload_resources: bool,
    previous_engine_index: usize,
    engine_cv: f32,

    previous_note: f32,
    trigger_state: bool,

    decay_envelope: DecayEnvelope,
    lpg_envelope: LpgEnvelope,

    trigger_delay: DelayLine<'a, f32, MAX_TRIGGER_DELAY>,

    out_post_processor: ChannelPostProcessor,
    aux_post_processor: ChannelPostProcessor,
}

impl<'a> Voice<'a> {
    pub fn new<T: GlobalAlloc>(buffer_allocator: &T, block_size: usize) -> Self {
        Self {
            additive_engine: AdditiveEngine::new(),
            bass_drum_engine: BassDrumEngine::new(),
            chiptune_engine: ChiptuneEngine::new(),
            chord_engine: ChordEngine::new(),
            fm_engine: FmEngine::new(),
            grain_engine: GrainEngine::new(),
            hihat_engine: HihatEngine::new(buffer_allocator, block_size),
            modal_engine: ModalEngine::new(buffer_allocator, block_size),
            noise_engine: NoiseEngine::new(buffer_allocator, block_size),
            particle_engine: ParticleEngine::new(buffer_allocator, block_size),
            phase_distortion_engine: PhaseDistortionEngine::new(buffer_allocator, block_size),
            six_op_engine: SixOpEngine::new(buffer_allocator, block_size),
            snare_drum_engine: SnareDrumEngine::new(),
            speech_engine: SpeechEngine::new(buffer_allocator, block_size),
            string_engine: StringEngine::new(buffer_allocator, block_size),
            string_machine_engine: StringMachineEngine::new(),
            swarm_engine: SwarmEngine::new(),
            virtual_analog_engine: VirtualAnalogEngine::new(buffer_allocator, block_size),
            virtual_analog_vcf_engine: VirtualAnalogVcfEngine::new(),
            waveshaping_engine: WaveshapingEngine::new(),
            wavetable_engine: WavetableEngine::new(),
            waveterrain_engine: WaveTerrainEngine::new(buffer_allocator, block_size),

            resources: Resources::default(),

            engine_quantizer: HysteresisQuantizer2::new(),
            reload_resources: false,
            previous_engine_index: 0,
            engine_cv: 0.0,

            previous_note: 0.0,
            trigger_state: false,

            decay_envelope: DecayEnvelope::new(),
            lpg_envelope: LpgEnvelope::new(),

            trigger_delay: DelayLine::new(
                allocate_buffer(buffer_allocator, MAX_TRIGGER_DELAY)
                    .unwrap()
                    .try_into()
                    .unwrap(),
            ),

            out_post_processor: ChannelPostProcessor::new(),
            aux_post_processor: ChannelPostProcessor::new(),
        }
    }

    pub fn init(&mut self) {
        for i in 0..NUM_ENGINES {
            self.get_engine(i).unwrap().0.init();
        }

        self.engine_quantizer.init(NUM_ENGINES as i32, 0.05, true);
        self.out_post_processor.init();
        self.aux_post_processor.init();
        self.decay_envelope.init();
        self.lpg_envelope.init();
    }

    #[inline]
    pub fn reload_resources(&mut self) {
        self.reload_resources = true;
    }

    #[inline]
    pub fn render(
        &mut self,
        patch: &Patch,
        modulations: &Modulations,
        out: &mut [f32],
        aux: &mut [f32],
    ) {
        // Trigger, LPG, internal envelope.

        // Delay trigger by 1ms to deal with sequencers or MIDI interfaces whose
        // CV out lags behind the GATE out.
        self.trigger_delay.write(modulations.trigger);
        let trigger_value = self.trigger_delay.read_with_delay(MAX_TRIGGER_DELAY);

        let previous_trigger_state = self.trigger_state;

        if !previous_trigger_state {
            if trigger_value > 0.3 {
                self.trigger_state = true;
                if !modulations.level_patched {
                    self.lpg_envelope.trigger();
                }
                self.decay_envelope.trigger();
                self.engine_cv = modulations.engine;
            }
        } else if trigger_value < 0.1 {
            self.trigger_state = false;
        }

        if !modulations.trigger_patched {
            self.engine_cv = modulations.engine;
        }

        // Engine selection.
        let mut engine_index =
            self.engine_quantizer
                .process_with_base(patch.engine as i32, self.engine_cv) as usize;
        engine_index = engine_index.clamp(0, NUM_ENGINES);

        if engine_index != self.previous_engine_index || self.reload_resources {
            match engine_index {
                2 => {
                    self.six_op_engine.load_syx_bank(self.resources.syx_bank_a);
                }
                3 => {
                    self.six_op_engine.load_syx_bank(self.resources.syx_bank_b);
                }
                4 => {
                    self.six_op_engine.load_syx_bank(self.resources.syx_bank_c);
                }
                5 => {
                    self.waveterrain_engine
                        .set_user_terrain(self.resources.user_wave_terrain);
                }
                13 => {
                    self.wavetable_engine
                        .set_wavetables(self.resources.wavetables);
                }
                _ => {}
            }

            let engine = self.get_engine(engine_index).unwrap().0;
            engine.reset();

            self.out_post_processor.reset();
            self.previous_engine_index = engine_index;
            self.reload_resources = false;
        }

        let mut p = EngineParameters::default();

        let rising_edge = self.trigger_state && !previous_trigger_state;
        let note = (modulations.note + self.previous_note) * 0.5;
        self.previous_note = modulations.note;

        if modulations.trigger_patched {
            p.trigger = if rising_edge {
                TriggerState::RisingEdge
            } else if self.trigger_state {
                TriggerState::High
            } else {
                TriggerState::Low
            };
        } else {
            p.trigger = TriggerState::Unpatched;
        }

        let short_decay = (200.0 * out.len() as f32) / SAMPLE_RATE
            * semitones_to_ratio(-96.0 * patch.decay.clamp(0.1, 1.0));

        self.decay_envelope.process(short_decay * 2.0);

        let compressed_level =
            (1.3 * modulations.level / (0.3 + modulations.level.abs())).clamp(0.0, 1.0);
        p.accent = if modulations.level_patched {
            compressed_level
        } else {
            0.8
        };

        let use_internal_envelope = modulations.trigger_patched;

        // Actual synthesis parameters.

        p.harmonics = patch.harmonics + modulations.harmonics;
        p.harmonics = p.harmonics.clamp(0.0, 1.0);

        let mut internal_envelope_amplitude = 1.0;
        let mut internal_envelope_amplitude_timbre = 1.0;

        if engine_index == 15 {
            internal_envelope_amplitude = 2.0 - p.harmonics * 6.0;
            internal_envelope_amplitude = internal_envelope_amplitude.clamp(0.0, 1.0);
            self.speech_engine.set_prosody_amount(
                if !modulations.trigger_patched || modulations.frequency_patched {
                    0.0
                } else {
                    patch.frequency_modulation_amount
                },
            );
            self.speech_engine.set_speed(
                if !modulations.trigger_patched || modulations.morph_patched {
                    0.0
                } else {
                    patch.morph_modulation_amount
                },
            );
        } else if engine_index == 7 {
            if modulations.trigger_patched && !modulations.timbre_patched {
                // Disable internal envelope on TIMBRE, and enable the envelope generator
                // built into the chiptune engine.
                // internal_envelope_amplitude_timbre = 0.0;
                // Envelope shape is determined by TIMBRE modulation amount. A minimum value
                // is forced to prevent infinite decay time.
                self.chiptune_engine
                    .set_envelope_shape(patch.timbre_modulation_amount.max(0.05));
            } else {
                self.chiptune_engine
                    .set_envelope_shape(chiptune_engine::NO_ENVELOPE);
            }
        }

        p.note = apply_modulations(
            patch.note + note,
            patch.frequency_modulation_amount,
            modulations.frequency_patched,
            modulations.frequency,
            use_internal_envelope,
            internal_envelope_amplitude
                * self.decay_envelope.value()
                * self.decay_envelope.value()
                * 48.0,
            1.0,
            -119.0,
            120.0,
        );

        p.timbre = apply_modulations(
            patch.timbre,
            patch.timbre_modulation_amount,
            modulations.timbre_patched,
            modulations.timbre,
            use_internal_envelope,
            internal_envelope_amplitude_timbre * self.decay_envelope.value(),
            0.0,
            0.0,
            1.0,
        );

        p.morph = apply_modulations(
            patch.morph,
            patch.morph_modulation_amount,
            modulations.morph_patched,
            modulations.morph,
            use_internal_envelope,
            internal_envelope_amplitude * self.decay_envelope.value(),
            0.0,
            0.0,
            1.0,
        );

        let engine = self.get_engine(engine_index).unwrap();
        let mut already_enveloped = engine.1;
        let out_gain = engine.2;
        let aux_gain = engine.3;

        engine.0.render(&p, out, aux, &mut already_enveloped);

        let lpg_bypass =
            already_enveloped || (!modulations.level_patched && !modulations.trigger_patched);

        // Compute LPG parameters.
        if !lpg_bypass {
            let hf = patch.lpg_colour;
            let decay_tail = (20.0 * out.len() as f32) / SAMPLE_RATE
                * semitones_to_ratio(-72.0 * patch.decay + 12.0 * hf)
                - short_decay;

            if modulations.level_patched {
                self.lpg_envelope
                    .process_lp(compressed_level, short_decay, decay_tail, hf);
            } else {
                let attack = note_to_frequency(p.note) * out.len() as f32 * 2.0;
                self.lpg_envelope
                    .process_ping(attack, short_decay, decay_tail, hf);
            }
        } else {
            self.lpg_envelope.init();
        }

        self.out_post_processor.process(
            out_gain,
            lpg_bypass,
            self.lpg_envelope.gain(),
            self.lpg_envelope.frequency(),
            self.lpg_envelope.hf_bleed(),
            out,
        );

        self.aux_post_processor.process(
            aux_gain,
            lpg_bypass,
            self.lpg_envelope.gain(),
            self.lpg_envelope.frequency(),
            self.lpg_envelope.hf_bleed(),
            aux,
        );
    }

    pub fn active_engine(&self) -> usize {
        self.previous_engine_index
    }

    /// Return reference to engine by index as well as additional parameters
    fn get_engine(&mut self, index: usize) -> Option<(&mut dyn Engine, bool, f32, f32)> {
        match index {
            0 => Some((&mut self.virtual_analog_vcf_engine, false, 1.0, 1.0)),
            1 => Some((&mut self.phase_distortion_engine, false, 0.7, 0.7)),
            2 => Some((&mut self.six_op_engine, true, 1.0, 1.0)),
            3 => Some((&mut self.six_op_engine, true, 1.0, 1.0)),
            4 => Some((&mut self.six_op_engine, true, 1.0, 1.0)),
            5 => Some((&mut self.waveterrain_engine, false, 0.7, 0.7)),
            6 => Some((&mut self.string_machine_engine, false, 0.8, 0.8)),
            7 => Some((&mut self.chiptune_engine, false, 0.5, 0.5)),
            8 => Some((&mut self.virtual_analog_engine, false, 0.8, 0.8)),
            9 => Some((&mut self.waveshaping_engine, false, 0.7, 0.6)),
            10 => Some((&mut self.fm_engine, false, 0.6, 0.6)),
            11 => Some((&mut self.grain_engine, false, 0.7, 0.6)),
            12 => Some((&mut self.additive_engine, false, 0.8, 0.8)),
            13 => Some((&mut self.wavetable_engine, false, 0.6, 0.6)),
            14 => Some((&mut self.chord_engine, false, 0.8, 0.8)),
            15 => Some((&mut self.speech_engine, false, -0.7, 0.8)),
            16 => Some((&mut self.swarm_engine, false, -3.0, 1.0)),
            17 => Some((&mut self.noise_engine, false, -1.0, -1.0)),
            18 => Some((&mut self.particle_engine, false, -2.0, 1.0)),
            19 => Some((&mut self.string_engine, true, -1.0, 0.8)),
            20 => Some((&mut self.modal_engine, true, -1.0, 0.8)),
            21 => Some((&mut self.bass_drum_engine, true, 0.8, 0.8)),
            22 => Some((&mut self.snare_drum_engine, true, 0.8, 0.8)),
            23 => Some((&mut self.hihat_engine, true, 0.8, 0.8)),
            _ => None,
        }
    }
}

#[derive(Debug, Default)]
pub struct ChannelPostProcessor {
    limiter: Limiter,
    lpg: LowPassGate,
}

impl ChannelPostProcessor {
    pub fn new() -> Self {
        Self {
            limiter: Limiter::new(),
            lpg: LowPassGate::new(),
        }
    }

    pub fn init(&mut self) {
        self.lpg.init();
        self.reset();
    }

    pub fn reset(&mut self) {
        self.limiter.init();
    }

    #[inline]
    pub fn process(
        &mut self,
        gain: f32,
        bypass_lpg: bool,
        low_pass_gate_gain: f32,
        low_pass_gate_frequency: f32,
        low_pass_gate_hf_bleed: f32,
        in_out: &mut [f32],
    ) {
        if gain < 0.0 {
            self.limiter.process(-gain, in_out);
        }

        let post_gain = if gain < 0.0 { 1.0 } else { gain };

        if !bypass_lpg {
            self.lpg.process_replacing(
                post_gain * low_pass_gate_gain,
                low_pass_gate_frequency,
                low_pass_gate_hf_bleed,
                in_out,
            );
        } else {
            for in_out_sample in in_out.iter_mut() {
                *in_out_sample *= post_gain;
                // *in_out_sample = soft_limit(*in_out_sample);
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    #[inline]
    pub fn process_to_i16(
        &mut self,
        gain: f32,
        bypass_lpg: bool,
        low_pass_gate_gain: f32,
        low_pass_gate_frequency: f32,
        low_pass_gate_hf_bleed: f32,
        in_: &mut [f32],
        out: &mut [i16],
    ) {
        if gain < 0.0 {
            self.limiter.process(-gain, in_);
        }

        let post_gain = (if gain < 0.0 { 1.0 } else { gain }) * -32767.0;

        if !bypass_lpg {
            self.lpg.process_to_i16(
                post_gain * low_pass_gate_gain,
                low_pass_gate_frequency,
                low_pass_gate_hf_bleed,
                in_,
                out,
                1,
            );
        } else {
            for (in_sample, out_sample) in in_.iter().zip(out.iter_mut()) {
                *out_sample = clip_16(1 + (*in_sample * post_gain) as i32) as i16;
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
#[inline]
fn apply_modulations(
    base_value: f32,
    mut modulation_amount: f32,
    use_external_modulation: bool,
    external_modulation: f32,
    use_internal_envelope: bool,
    envelope: f32,
    default_internal_modulation: f32,
    minimum_value: f32,
    maximum_value: f32,
) -> f32 {
    let mut value = base_value;
    modulation_amount *= f32::max(modulation_amount.abs() - 0.05, 0.05);
    modulation_amount *= 1.05;

    let modulation = if use_external_modulation {
        external_modulation
    } else if use_internal_envelope {
        envelope
    } else {
        default_internal_modulation
    };

    value += modulation_amount * modulation;
    value = value.clamp(minimum_value, maximum_value);

    value
}
