// Open Source implementation of Audio Processing Technology codec (aptX)
// Copyright (C) 2017       Aurelien Jacobs <aurel@gnuage.org>
// Copyright (C) 2018-2021  Pali Rohár <pali.rohar@gmail.com>
// Rust version             Vitor Ramos <ramos.vitor89@gmail.com>
//
// Read README file for license details.  Due to license abuse
// this library must not be used in any Freedesktop project.
//
// This library is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This library is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this library.  If not, see <http://www.gnu.org/licenses/>.

mod aptx_table;
use aptx_table::{
    HD_INVERT_QUANTIZE_DITHER_FACTORS_HF, HD_INVERT_QUANTIZE_DITHER_FACTORS_LF,
    HD_INVERT_QUANTIZE_DITHER_FACTORS_MHF, HD_INVERT_QUANTIZE_DITHER_FACTORS_MLF,
    HD_QUANTIZE_DITHER_FACTORS_HF, HD_QUANTIZE_DITHER_FACTORS_LF, HD_QUANTIZE_DITHER_FACTORS_MHF,
    HD_QUANTIZE_DITHER_FACTORS_MLF, HD_QUANTIZE_FACTOR_SELECT_OFFSET_HF,
    HD_QUANTIZE_FACTOR_SELECT_OFFSET_LF, HD_QUANTIZE_FACTOR_SELECT_OFFSET_MHF,
    HD_QUANTIZE_FACTOR_SELECT_OFFSET_MLF, HD_QUANTIZE_INTERVALS_HF, HD_QUANTIZE_INTERVALS_LF,
    HD_QUANTIZE_INTERVALS_MHF, HD_QUANTIZE_INTERVALS_MLF, INVERT_QUANTIZE_DITHER_FACTORS_HF,
    INVERT_QUANTIZE_DITHER_FACTORS_LF, INVERT_QUANTIZE_DITHER_FACTORS_MHF,
    INVERT_QUANTIZE_DITHER_FACTORS_MLF, QUANTIZE_DITHER_FACTORS_HF, QUANTIZE_DITHER_FACTORS_LF,
    QUANTIZE_DITHER_FACTORS_MHF, QUANTIZE_DITHER_FACTORS_MLF, QUANTIZE_FACTOR_SELECT_OFFSET_HF,
    QUANTIZE_FACTOR_SELECT_OFFSET_LF, QUANTIZE_FACTOR_SELECT_OFFSET_MHF,
    QUANTIZE_FACTOR_SELECT_OFFSET_MLF, QUANTIZE_INTERVALS_HF, QUANTIZE_INTERVALS_LF,
    QUANTIZE_INTERVALS_MHF, QUANTIZE_INTERVALS_MLF,
};

#[macro_export]
macro_rules! diffsign {
    ($x:expr, $y:expr) => {
        (($x > $y) as i32 - ($x < $y) as i32)
    };
}

#[inline]
const fn clip_intp2(a: i32, p: u32) -> i32 {
    if ((a as u32).wrapping_add(1_u32 << p) & !((2_u32 << p) - 1)) != 0 {
        (a >> 31) ^ (1_i32 << p).wrapping_sub(1)
    } else {
        a
    }
}

#[inline]
const fn sign_extend(val: i32, bits: u32) -> i32 {
    let shift = 8 * std::mem::size_of::<i32>() as u32 - bits;
    let v = ((val as u32) << shift) as i32;
    v >> shift
}

#[inline]
const fn rshift32(value: i32, shift: u32) -> i32 {
    let rounding = 1 << (shift - 1);
    let mask = (1 << (shift + 1)) - 1;
    ((value + rounding) >> shift) - ((value & mask) == rounding) as i32
}

#[inline]
const fn rshift64(value: i64, shift: u32) -> i64 {
    let rounding = 1_i64 << (shift - 1);
    let mask = (1_i64 << (shift + 1)) - 1;
    ((value + rounding) >> shift) - (((value & mask) == rounding) as i64)
}

#[inline]
const fn rshift32_clip24(value: i32, shift: u32) -> i32 {
    clip_intp2(rshift32(value, shift), 23)
}

#[inline]
const fn rshift64_clip24(value: i64, shift: u32) -> i32 {
    clip_intp2(rshift64(value, shift) as i32, 23)
}

#[inline]
const fn aptx_bin_search(value: i32, factor: i32, intervals: &[i32]) -> i32 {
    let mut idx = 0;
    let mut i = intervals.len() >> 1;

    while i > 0 {
        if (factor as i64 * intervals[idx + i] as i64) <= ((value as i64) << 24) {
            idx += i;
        }
        i >>= 1;
    }

    idx as i32
}

pub struct AptxTables {
    quantize_intervals: &'static [i32],
    invert_quantize_dither_factors: &'static [i32],
    quantize_dither_factors: &'static [i32],
    quantize_factor_select_offset: &'static [i16],
    factor_max: i32,
    prediction_order: i32,
}

#[derive(Default)]
struct AptxFilterSignal {
    buffer: [i32; 2 * Self::FILTER_TAPS],
    pos: u8,
}

#[derive(Default)]
struct AptxQmfAnalysis {
    outer_filter_signal: [AptxFilterSignal; Self::NB_FILTERS],
    inner_filter_signal: [[AptxFilterSignal; Self::NB_FILTERS]; Self::NB_FILTERS],
}

#[derive(Default)]
struct AptxQuantize {
    quantized_sample: i32,
    quantized_sample_parity_change: i32,
    error: i32,
}

#[derive(Default)]
struct AptxInvertQuantize {
    quantization_factor: i32,
    factor_select: i32,
    reconstructed_difference: i32,
}

struct AptxPrediction {
    prev_sign: [i32; 2],
    s_weight: [i32; 2],
    d_weight: [i32; 24],
    pos: i32,
    reconstructed_differences: [i32; 48],
    previous_reconstructed_sample: i32,
    predicted_difference: i32,
    predicted_sample: i32,
}

impl Default for AptxPrediction {
    fn default() -> Self {
        Self {
            prev_sign: Default::default(),
            s_weight: Default::default(),
            d_weight: Default::default(),
            pos: Default::default(),
            reconstructed_differences: [0i32; 48],
            previous_reconstructed_sample: Default::default(),
            predicted_difference: Default::default(),
            predicted_sample: Default::default(),
        }
    }
}

#[derive(Default)]
struct AptxChannel {
    codeword_history: i32,
    dither_parity: i32,
    qmf: AptxQmfAnalysis,
    dither: [i32; Self::NB_SUBBANDS],
    quantize: [AptxQuantize; Self::NB_SUBBANDS],
    invert_quantize: [AptxInvertQuantize; Self::NB_SUBBANDS],
    prediction: [AptxPrediction; Self::NB_SUBBANDS],
    samples: [i32; Self::NB_SUBBANDS],
}

pub struct AptxContext {
    decode_sync_packets: usize,
    decode_dropped: usize,
    channels: [AptxChannel; 2],
    hd: bool,
    sync_idx: u8,
    encode_remaining: u8,
    decode_skip_leading: u8,
    decode_sync_buffer_len: u8,
    decode_sync_buffer: [u8; 6],
}

impl Default for AptxContext {
    fn default() -> Self {
        let mut ctx = AptxContext {
            decode_sync_packets: Default::default(),
            decode_dropped: Default::default(),
            channels: Default::default(),
            hd: Default::default(),
            sync_idx: Default::default(),
            encode_remaining: ((Self::LATENCY_SAMPLES + 3) / 4) as u8,
            decode_skip_leading: ((Self::LATENCY_SAMPLES + 3) / 4) as u8,
            decode_sync_buffer_len: Default::default(),
            decode_sync_buffer: Default::default(),
        };
        for channel in &mut ctx.channels {
            for prediction in &mut channel.prediction {
                prediction.prev_sign[0] = 1;
                prediction.prev_sign[1] = 1;
            }
        }
        ctx
    }
}

impl AptxFilterSignal {
    pub const FILTER_TAPS: usize = 16;

    fn qmf_filter_signal_push(&mut self, sample: i32) {
        self.buffer[self.pos as usize] = sample;
        self.buffer[(self.pos + Self::FILTER_TAPS as u8) as usize] = sample;
        self.pos = (self.pos + 1) & (Self::FILTER_TAPS as u8 - 1);
    }

    fn qmf_convolution(&self, coeffs: &[i32; Self::FILTER_TAPS], shift: u32) -> i32 {
        let sig = &self.buffer[self.pos as usize..];
        let mut e: i64 = 0;

        for i in 0..Self::FILTER_TAPS {
            e += sig[i] as i64 * coeffs[i] as i64;
        }

        rshift64_clip24(e, shift)
    }
}

impl AptxQmfAnalysis {
    pub const NB_FILTERS: usize = 2;
    const APTX_QMF_OUTER_COEFFS: [[i32; AptxFilterSignal::FILTER_TAPS]; Self::NB_FILTERS] = [
        [
            730, -413, -9611, 43626, -121026, 269973, -585547, 2801966, 697128, -160481, 27611,
            8478, -10043, 3511, 688, -897,
        ],
        [
            -897, 688, 3511, -10043, 8478, 27611, -160481, 697128, 2801966, -585547, 269973,
            -121026, 43626, -9611, -413, 730,
        ],
    ];

    const APTX_QMF_INNER_COEFFS: [[i32; AptxFilterSignal::FILTER_TAPS]; Self::NB_FILTERS] = [
        [
            1033, -584, -13592, 61697, -171156, 381799, -828088, 3962579, 985888, -226954, 39048,
            11990, -14203, 4966, 973, -1268,
        ],
        [
            -1268, 973, 4966, -14203, 11990, 39048, -226954, 985888, 3962579, -828088, 381799,
            -171156, 61697, -13592, -584, 1033,
        ],
    ];

    fn qmf_polyphase_analysis(
        signal: &mut [AptxFilterSignal; Self::NB_FILTERS],
        coeffs: &[[i32; AptxFilterSignal::FILTER_TAPS]; Self::NB_FILTERS],
        shift: u32,
        samples: &[i32; Self::NB_FILTERS],
        low_subband_output: &mut i32,
        high_subband_output: &mut i32,
    ) {
        let mut subbands = [0; Self::NB_FILTERS];

        for i in 0..Self::NB_FILTERS {
            signal[i].qmf_filter_signal_push(samples[Self::NB_FILTERS - 1 - i]);
            subbands[i] = signal[i].qmf_convolution(&coeffs[i], shift);
        }

        *low_subband_output = clip_intp2(subbands[0] + subbands[1], 23);
        *high_subband_output = clip_intp2(subbands[0] - subbands[1], 23);
    }

    fn qmf_polyphase_synthesis(
        signal: &mut [AptxFilterSignal; Self::NB_FILTERS],
        coeffs: &[[i32; AptxFilterSignal::FILTER_TAPS]; Self::NB_FILTERS],
        shift: u32,
        low_subband_input: i32,
        high_subband_input: i32,
        samples: &mut [i32],
    ) {
        let mut subbands: [i32; Self::NB_FILTERS] = [0; Self::NB_FILTERS];

        subbands[0] = low_subband_input + high_subband_input;
        subbands[1] = low_subband_input - high_subband_input;

        for i in 0..Self::NB_FILTERS {
            signal[i].qmf_filter_signal_push(subbands[1 - i]);
            samples[i] = signal[i].qmf_convolution(&coeffs[i], shift);
        }
    }

    fn tree_synthesis(
        &mut self,
        subband_samples: &[i32; AptxChannel::NB_SUBBANDS],
        samples: &mut [i32; AptxChannel::NB_SUBBANDS],
    ) {
        let mut intermediate_samples = [0i32; AptxChannel::NB_SUBBANDS];

        for i in 0..2 {
            Self::qmf_polyphase_synthesis(
                &mut self.inner_filter_signal[i],
                &Self::APTX_QMF_INNER_COEFFS,
                22,
                subband_samples[2 * i],
                subband_samples[2 * i + 1],
                &mut intermediate_samples[(2 * i)..],
            );
        }

        for i in 0..2 {
            Self::qmf_polyphase_synthesis(
                &mut self.outer_filter_signal,
                &Self::APTX_QMF_OUTER_COEFFS,
                21,
                intermediate_samples[i],
                intermediate_samples[2 + i],
                &mut samples[(2 * i)..],
            );
        }
    }

    fn tree_analysis(
        &mut self,
        samples: &[i32; AptxChannel::NB_SUBBANDS],
        subband_samples: &mut [i32; AptxChannel::NB_SUBBANDS],
    ) {
        let mut intermediate_samples = [0i32; AptxChannel::NB_SUBBANDS];

        let (a, b) = intermediate_samples.split_at_mut(2);
        for i in 0..2 {
            Self::qmf_polyphase_analysis(
                &mut self.outer_filter_signal,
                &Self::APTX_QMF_OUTER_COEFFS,
                23,
                &samples[(2 * i)..(2 * i + 2)].try_into().unwrap(),
                &mut a[i],
                &mut b[i],
            );
        }

        for i in 0..2 {
            let (a, b) = subband_samples[2 * i..].split_at_mut(1);
            Self::qmf_polyphase_analysis(
                &mut self.inner_filter_signal[i],
                &Self::APTX_QMF_INNER_COEFFS,
                23,
                &intermediate_samples[(2 * i)..(2 * i + 2)]
                    .try_into()
                    .unwrap(),
                &mut a[0],
                &mut b[0],
            );
        }
    }
}

impl AptxQuantize {
    fn quantize_difference(
        &mut self,
        sample_difference: i32,
        dither: i32,
        quantization_factor: i32,
        tables: &AptxTables,
    ) {
        let intervals = tables.quantize_intervals;
        let mut quantized_sample;
        let mut parity_change;
        let mut sample_difference_abs;

        sample_difference_abs = if sample_difference < 0 {
            -sample_difference
        } else {
            sample_difference
        };

        if sample_difference_abs > ((1 << 23) - 1) {
            sample_difference_abs = (1 << 23) - 1;
        }

        quantized_sample =
            aptx_bin_search(sample_difference_abs >> 4, quantization_factor, intervals);

        let d = rshift32_clip24(((dither as i64 * dither as i64) >> 32) as i32, 7) - (1 << 23);
        let d = rshift64(
            d as i64 * tables.quantize_dither_factors[quantized_sample as usize] as i64,
            23,
        ) as i32;

        let mean =
            (intervals[quantized_sample as usize + 1] + intervals[quantized_sample as usize]) / 2;
        let interval = (intervals[quantized_sample as usize + 1]
            - intervals[quantized_sample as usize])
            * (-((sample_difference < 0) as i32) | 1);

        let a = dither as i64 * interval as i64;
        let b = ((clip_intp2(mean + d, 23) as i64) << 32) as i64;
        let dithered_sample = rshift64_clip24(a + b, 32);
        let error = ((sample_difference_abs as i64) << 20)
            - (dithered_sample as i64 * quantization_factor as i64);
        self.error = rshift64(error, 23) as i32;
        if self.error < 0 {
            self.error = -self.error;
        }

        parity_change = quantized_sample;
        if error < 0 {
            quantized_sample -= 1;
        } else {
            parity_change -= 1;
        }

        let inv = -((sample_difference < 0) as i32);
        self.quantized_sample = quantized_sample ^ inv;
        self.quantized_sample_parity_change = parity_change ^ inv;
    }
}

impl AptxPrediction {
    fn reconstructed_differences_update(
        &mut self,
        reconstructed_difference: i32,
        order: i32,
    ) -> *mut i32 {
        let mut p = self.pos as usize;
        let (rd1, rd2) = self.reconstructed_differences.split_at_mut(order as usize);
        rd1[p] = rd2[p];
        p = (p + 1) % order as usize;
        self.pos = p as i32;
        rd2[p] = reconstructed_difference;
        &mut rd2[p]
    }

    fn prediction_filtering(&mut self, reconstructed_difference: i32, order: i32) {
        let mut srd;
        let mut predicted_difference = 0i64;

        let reconstructed_sample = clip_intp2(reconstructed_difference + self.predicted_sample, 23);
        let predictor = clip_intp2(
            ((self.s_weight[0] as i64 * self.previous_reconstructed_sample as i64
                + self.s_weight[1] as i64 * reconstructed_sample as i64)
                >> 22) as i32,
            23,
        );
        self.previous_reconstructed_sample = reconstructed_sample;

        let reconstructed_differences_ptr =
            self.reconstructed_differences_update(reconstructed_difference, order);
        let srd0 = diffsign!(reconstructed_difference, 0) * (1 << 23);
        for i in 0..order {
            srd = (unsafe { *reconstructed_differences_ptr.wrapping_sub((i + 1) as usize) } >> 31)
                | 1;
            self.d_weight[i as usize] -= rshift32(self.d_weight[i as usize] - srd * srd0, 8);
            predicted_difference +=
                unsafe { *reconstructed_differences_ptr.wrapping_sub(i as usize) } as i64
                    * self.d_weight[i as usize] as i64;
        }

        self.predicted_difference = clip_intp2((predicted_difference >> 22) as i32, 23);
        self.predicted_sample = clip_intp2(predictor + self.predicted_difference, 23);
    }
}

impl AptxInvertQuantize {
    const QUANTIZATION_FACTORS: [i16; 32] = [
        2048, 2093, 2139, 2186, 2233, 2282, 2332, 2383, 2435, 2489, 2543, 2599, 2656, 2714, 2774,
        2834, 2896, 2960, 3025, 3091, 3158, 3228, 3298, 3371, 3444, 3520, 3597, 3676, 3756, 3838,
        3922, 4008,
    ];

    fn process_subband(
        &mut self,
        prediction: &mut AptxPrediction,
        quantized_sample: i32,
        dither: i32,
        tables: &AptxTables,
    ) {
        let mut same_sign: [i32; 2] = [0; 2];
        let mut weight: [i32; 2] = [0; 2];
        let mut sw1;
        let mut range;

        self.invert_quantization(quantized_sample, dither, tables);

        let sign = diffsign!(
            self.reconstructed_difference,
            -prediction.predicted_difference
        );
        same_sign[0] = sign * prediction.prev_sign[0];
        same_sign[1] = sign * prediction.prev_sign[1];
        prediction.prev_sign[0] = prediction.prev_sign[1];
        prediction.prev_sign[1] = sign | 1;

        range = 0x100000;
        sw1 = rshift32(-same_sign[1] * prediction.s_weight[1], 1);
        sw1 = (sw1.clamp(-range, range) & !0xF) * 16;

        range = 0x300000;
        weight[0] = 254 * prediction.s_weight[0] + 0x800000 * same_sign[0] + sw1;
        prediction.s_weight[0] = rshift32(weight[0], 8).clamp(-range, range);

        range = 0x3C0000 - prediction.s_weight[0];
        weight[1] = 255 * prediction.s_weight[1] + 0xC00000 * same_sign[1];
        prediction.s_weight[1] = rshift32(weight[1], 8).clamp(-range, range);

        prediction.prediction_filtering(self.reconstructed_difference, tables.prediction_order);
    }

    fn invert_quantization(&mut self, quantized_sample: i32, dither: i32, tables: &AptxTables) {
        let mut qr;
        let mut idx;

        let mut factor_select;

        idx = (quantized_sample ^ -((quantized_sample < 0) as i32)) + 1;
        qr = tables.quantize_intervals[idx as usize] / 2;
        if quantized_sample < 0 {
            qr = -qr;
        }

        qr = rshift64_clip24(
            ((qr as i64) << 32)
                + (dither as i64 * tables.invert_quantize_dither_factors[idx as usize] as i64),
            32,
        );
        self.reconstructed_difference =
            ((self.quantization_factor as i64 * qr as i64) >> 19) as i32;

        factor_select = 32620 * self.factor_select;
        factor_select = rshift32(
            factor_select + (tables.quantize_factor_select_offset[idx as usize] as i32 * (1 << 15)),
            15,
        );
        self.factor_select = factor_select.clamp(0, tables.factor_max);

        idx = (self.factor_select & 0xFF) >> 3;
        let shift = (tables.factor_max - self.factor_select) >> 8;
        self.quantization_factor =
            ((Self::QUANTIZATION_FACTORS[idx as usize] as i32) << 11) >> shift;
    }
}

impl AptxChannel {
    const NB_SUBBANDS: usize = 4;
    const ALL_TABLES: [[AptxTables; Self::NB_SUBBANDS]; 2] = [
        [
            AptxTables {
                quantize_intervals: &QUANTIZE_INTERVALS_LF,
                invert_quantize_dither_factors: &INVERT_QUANTIZE_DITHER_FACTORS_LF,
                quantize_dither_factors: &QUANTIZE_DITHER_FACTORS_LF,
                quantize_factor_select_offset: &QUANTIZE_FACTOR_SELECT_OFFSET_LF,
                factor_max: 0x11FF,
                prediction_order: 24,
            },
            AptxTables {
                quantize_intervals: &QUANTIZE_INTERVALS_MLF,
                invert_quantize_dither_factors: &INVERT_QUANTIZE_DITHER_FACTORS_MLF,
                quantize_dither_factors: &QUANTIZE_DITHER_FACTORS_MLF,
                quantize_factor_select_offset: &QUANTIZE_FACTOR_SELECT_OFFSET_MLF,
                factor_max: 0x14FF,
                prediction_order: 12,
            },
            AptxTables {
                quantize_intervals: &QUANTIZE_INTERVALS_MHF,
                invert_quantize_dither_factors: &INVERT_QUANTIZE_DITHER_FACTORS_MHF,
                quantize_dither_factors: &QUANTIZE_DITHER_FACTORS_MHF,
                quantize_factor_select_offset: &QUANTIZE_FACTOR_SELECT_OFFSET_MHF,
                factor_max: 0x16FF,
                prediction_order: 6,
            },
            AptxTables {
                quantize_intervals: &QUANTIZE_INTERVALS_HF,
                invert_quantize_dither_factors: &INVERT_QUANTIZE_DITHER_FACTORS_HF,
                quantize_dither_factors: &QUANTIZE_DITHER_FACTORS_HF,
                quantize_factor_select_offset: &QUANTIZE_FACTOR_SELECT_OFFSET_HF,
                factor_max: 0x15FF,
                prediction_order: 12,
            },
        ],
        [
            AptxTables {
                quantize_intervals: &HD_QUANTIZE_INTERVALS_LF,
                invert_quantize_dither_factors: &HD_INVERT_QUANTIZE_DITHER_FACTORS_LF,
                quantize_dither_factors: &HD_QUANTIZE_DITHER_FACTORS_LF,
                quantize_factor_select_offset: &HD_QUANTIZE_FACTOR_SELECT_OFFSET_LF,
                factor_max: 0x11FF,
                prediction_order: 24,
            },
            AptxTables {
                quantize_intervals: &HD_QUANTIZE_INTERVALS_MLF,
                invert_quantize_dither_factors: &HD_INVERT_QUANTIZE_DITHER_FACTORS_MLF,
                quantize_dither_factors: &HD_QUANTIZE_DITHER_FACTORS_MLF,
                quantize_factor_select_offset: &HD_QUANTIZE_FACTOR_SELECT_OFFSET_MLF,
                factor_max: 0x14FF,
                prediction_order: 12,
            },
            AptxTables {
                quantize_intervals: &HD_QUANTIZE_INTERVALS_MHF,
                invert_quantize_dither_factors: &HD_INVERT_QUANTIZE_DITHER_FACTORS_MHF,
                quantize_dither_factors: &HD_QUANTIZE_DITHER_FACTORS_MHF,
                quantize_factor_select_offset: &HD_QUANTIZE_FACTOR_SELECT_OFFSET_MHF,
                factor_max: 0x16FF,
                prediction_order: 6,
            },
            AptxTables {
                quantize_intervals: &HD_QUANTIZE_INTERVALS_HF,
                invert_quantize_dither_factors: &HD_INVERT_QUANTIZE_DITHER_FACTORS_HF,
                quantize_dither_factors: &HD_QUANTIZE_DITHER_FACTORS_HF,
                quantize_factor_select_offset: &HD_QUANTIZE_FACTOR_SELECT_OFFSET_HF,
                factor_max: 0x15FF,
                prediction_order: 12,
            },
        ],
    ];

    fn update_codeword_history(&mut self) {
        let cw = (self.quantize[0].quantized_sample & 3)
            + ((self.quantize[1].quantized_sample & 2) << 1)
            + ((self.quantize[2].quantized_sample & 1) << 3);
        self.codeword_history = (cw << 8) + (self.codeword_history << 4);
    }

    fn generate_dither(&mut self) {
        self.update_codeword_history();

        let m: i64 = 5184443 * (self.codeword_history >> 7) as i64;
        let d: i32 = ((m * 4) + (m >> 22)) as i32;
        for subband in 0..Self::NB_SUBBANDS {
            self.dither[subband] = d << (23 - 5 * subband);
        }
        self.dither_parity = (d >> 25) & 1;
    }

    fn encode_channel(&mut self, hd: bool) {
        let mut subband_samples = [0i32; Self::NB_SUBBANDS];
        let mut diff;

        self.qmf.tree_analysis(&self.samples, &mut subband_samples);
        self.generate_dither();

        for (idx, subband) in subband_samples.iter().enumerate() {
            diff = clip_intp2(subband - self.prediction[idx].predicted_sample, 23);
            self.quantize[idx].quantize_difference(
                diff,
                self.dither[idx],
                self.invert_quantize[idx].quantization_factor,
                &Self::ALL_TABLES[hd as usize][idx],
            );
        }
    }

    fn decode_channel(&mut self) {
        let mut subband_samples = [0i32; Self::NB_SUBBANDS];

        for (idx, subband) in subband_samples.iter_mut().enumerate() {
            *subband = self.prediction[idx].previous_reconstructed_sample;
        }
        self.qmf.tree_synthesis(&subband_samples, &mut self.samples);
    }

    fn invert_quantize_and_prediction(&mut self, hd: bool) {
        for subband in 0..Self::NB_SUBBANDS {
            self.invert_quantize[subband].process_subband(
                &mut self.prediction[subband],
                self.quantize[subband].quantized_sample,
                self.dither[subband],
                &Self::ALL_TABLES[hd as usize][subband],
            );
        }
    }

    fn quantized_parity(&self) -> i32 {
        let mut parity = self.dither_parity;
        for quantize in &self.quantize {
            parity ^= quantize.quantized_sample;
        }
        parity & 1
    }

    fn pack_codeword(&mut self) -> u16 {
        let parity = self.quantized_parity();
        (((self.quantize[3].quantized_sample & 0x06 | parity) << 13)
            | ((self.quantize[2].quantized_sample & 0x03) << 11)
            | ((self.quantize[1].quantized_sample & 0x0F) << 7)
            | (self.quantize[0].quantized_sample & 0x7F)) as u16
    }

    fn pack_codewordhd(&self) -> u32 {
        let parity = self.quantized_parity();
        (((self.quantize[3].quantized_sample & 0x01E | parity) << 19)
            | ((self.quantize[2].quantized_sample & 0x00F) << 15)
            | ((self.quantize[1].quantized_sample & 0x03F) << 9)
            | (self.quantize[0].quantized_sample & 0x1FF)) as u32
    }

    fn unpack_codeword(&mut self, codeword: u16) {
        self.quantize[0].quantized_sample = sign_extend(codeword as i32, 7);
        self.quantize[1].quantized_sample = sign_extend((codeword >> 7) as i32, 4);
        self.quantize[2].quantized_sample = sign_extend((codeword >> 11) as i32, 2);
        self.quantize[3].quantized_sample = sign_extend((codeword >> 13) as i32, 3);
        self.quantize[3].quantized_sample =
            (self.quantize[3].quantized_sample & !1) | self.quantized_parity();
    }

    fn unpack_codewordhd(&mut self, codeword: u32) {
        self.quantize[0].quantized_sample = sign_extend(codeword as i32, 9);
        self.quantize[1].quantized_sample = sign_extend((codeword >> 9) as i32, 6);
        self.quantize[2].quantized_sample = sign_extend((codeword >> 15) as i32, 4);
        self.quantize[3].quantized_sample = sign_extend((codeword >> 19) as i32, 5);
        self.quantize[3].quantized_sample =
            (self.quantize[3].quantized_sample & !1) | self.quantized_parity();
    }
}

impl AptxContext {
    const LEFT: usize = 0;
    const RIGHT: usize = 1;
    const LATENCY_SAMPLES: usize = 90;

    pub fn new(hd: bool) -> Box<AptxContext> {
        Box::new(AptxContext {
            hd,
            ..Default::default()
        })
    }

    pub fn reset(&mut self) {
        *self = AptxContext {
            hd: self.hd,
            ..Default::default()
        };
    }

    fn check_parity(&mut self) -> i32 {
        let parity = self.channels[Self::LEFT].quantized_parity()
            ^ self.channels[Self::RIGHT].quantized_parity();
        let eighth = self.sync_idx == 7;

        self.sync_idx = (self.sync_idx + 1) & 7;
        parity ^ eighth as i32
    }

    fn insert_sync(&mut self) {
        if self.check_parity() != 0 {
            let map = [1, 2, 0, 3];
            let (mut mi, mut mj, mut min_error) = (
                self.channels.len() - 1,
                map[0],
                self.channels.last().unwrap().quantize[map[0]].error,
            );
            for (i, channel) in self.channels.iter().rev().enumerate() {
                for j in map {
                    if channel.quantize[j].error < min_error {
                        mi = i;
                        mj = j;
                        min_error = channel.quantize[j].error;
                    }
                }
            }
            self.channels[mi].quantize[mj].quantized_sample =
                self.channels[mi].quantize[mj].quantized_sample_parity_change;
        }
    }

    fn encode_samples(&mut self, output: &mut [u8]) {
        for channel in &mut self.channels {
            channel.encode_channel(self.hd);
        }
        self.insert_sync();
        for (idx, channel) in self.channels.iter_mut().enumerate() {
            channel.invert_quantize_and_prediction(self.hd);
            if self.hd {
                let codeword = channel.pack_codewordhd();
                output[3 * idx] = (codeword >> 16) as u8;
                output[3 * idx + 1] = (codeword >> 8) as u8;
                output[3 * idx + 2] = codeword as u8;
            } else {
                let codeword = channel.pack_codeword();
                output[2 * idx] = (codeword >> 8) as u8;
                output[2 * idx + 1] = codeword as u8;
            }
        }
    }

    fn decode_samples(&mut self, input: &[u8]) -> i32 {
        for (idx, channel) in self.channels.iter_mut().enumerate() {
            channel.generate_dither();
            if self.hd {
                channel.unpack_codewordhd(
                    (input[3 * idx] as u32) << 16
                        | (input[3 * idx + 1] as u32) << 8
                        | input[3 * idx + 2] as u32,
                );
            } else {
                channel.unpack_codeword((input[2 * idx] as u16) << 8 | input[2 * idx + 1] as u16);
            }
            channel.invert_quantize_and_prediction(self.hd);
        }

        let ret = self.check_parity();
        for channel in &mut self.channels {
            channel.decode_channel();
        }
        ret
    }

    fn reset_decode_sync(&mut self) {
        let decode_dropped = self.decode_dropped;
        let decode_sync_packets = self.decode_sync_packets;
        let decode_sync_buffer_len = self.decode_sync_buffer_len;
        let decode_sync_buffer = self.decode_sync_buffer;
        self.reset();
        self.decode_sync_buffer.copy_from_slice(&decode_sync_buffer);
        self.decode_sync_buffer_len = decode_sync_buffer_len;
        self.decode_sync_packets = decode_sync_packets;
        self.decode_dropped = decode_dropped;
    }

    /// Encodes a sequence of pcm_16_le or pcm_24_le (HD) audio samples from the input buffer into
    /// aptx format with 4:1 compression ratio and stores it in the output buffer.
    ///
    /// # Parameters
    ///
    /// * `input` - A slice of bytes representing the input audio samples to be encoded.
    ///             The input is expected to be in interleaved format, with each sample consisting of multiple bytes.
    /// * `output` - A mutable slice of bytes where the encoded output will be stored.
    /// * `written` - A mutable reference to a `usize` variable that will be updated with the
    ///               number of bytes written to the output buffer.
    ///
    /// # Returns
    ///
    /// The number of bytes read from the input buffer.
    ///
    /// # Example
    ///
    /// ```
    /// let mut encoder = AptxContext::new(false);
    /// let input_data: Vec<u8> = vec![0; 256*4];
    /// let mut output_data: Vec<u8> = vec![0; 256];
    /// let mut bytes_written: usize = 0;
    ///
    /// let bytes_read = encoder.encode(&input_data, &mut output_data, &mut bytes_written);
    ///
    /// println!("Bytes read from input: {}", bytes_read);
    /// println!("Bytes written to output: {}", bytes_written);
    /// ```
    ///
    pub fn encode(&mut self, input: &[u8], output: &mut [u8], written: &mut usize) -> usize {
        let sample_size = if self.hd { 6 } else { 4 };
        let mut ipos = 0;
        let mut opos = 0;
        let input_size = input.len();
        let output_size = output.len();

        while ipos + sample_size / 2 * self.channels.len() * 4 <= input_size
            && opos + sample_size <= output_size
        {
            for sample in 0..4 {
                for channel in self.channels.iter_mut() {
                    if self.hd {
                        channel.samples[sample] = (input[ipos] as i32)
                            | ((input[ipos + 1] as i32) << 8)
                            | ((input[ipos + 2] as i8 as i32) << 16);
                    } else {
                        channel.samples[sample] =
                            ((input[ipos] as i32) << 8) | ((input[ipos + 1] as i8 as i32) << 16);
                    }
                    ipos += sample_size / 2;
                }
            }
            self.encode_samples(&mut output[opos..]);
            opos += sample_size;
        }

        *written = opos;
        ipos
    }

    pub fn encode_finish(&mut self, output: &mut [u8], written: &mut usize) -> i32 {
        let sample_size = if self.hd { 6 } else { 4 };
        let mut opos = 0;
        let output_size = output.len();

        if self.encode_remaining == 0 {
            *written = 0;
            return 1;
        }

        while self.encode_remaining > 0 && opos + sample_size <= output_size {
            self.encode_samples(&mut output[opos..]);
            self.encode_remaining -= 1;
            opos += sample_size;
        }

        *written = opos;

        if self.encode_remaining > 0 {
            return 0;
        }

        self.reset();
        1
    }

    /// Decodes a sequence of aptX encoded audio samples from the input buffer into
    /// pcm_16_le or pcm_24_le (HD) format and stores it in the output buffer.
    ///
    /// # Parameters
    ///
    /// * `input` - A slice of bytes representing the input aptX encoded audio samples to be decoded.
    ///             The input is expected to be in aptX format, with each sample consisting of multiple bytes.
    /// * `output` - A mutable slice of bytes where the decoded output will be stored.
    ///              The output will be in interleaved format, with each sample consisting of multiple bytes.
    /// * `written` - A mutable reference to a `usize` variable that will be updated with the
    ///               number of bytes written to the output buffer.
    ///
    /// # Returns
    ///
    /// The number of bytes read from the input buffer.
    ///
    /// # Example
    ///
    /// ```
    /// let mut decoder = AptxContext::new(false);
    /// let input_data: Vec<u8> = vec![0; 256];
    /// let mut output_data: Vec<u8> = vec![0; 256 * 4];
    /// let mut bytes_written: usize = 0;
    ///
    /// let bytes_read = decoder.decode(&input_data, &mut output_data, &mut bytes_written);
    ///
    /// println!("Bytes read from input: {}", bytes_read);
    /// println!("Bytes written to output: {}", bytes_written);
    /// ```
    ///
    pub fn decode(&mut self, input: &[u8], output: &mut [u8], written: &mut usize) -> usize {
        let sample_size = if self.hd { 6 } else { 4 };
        let mut ipos = 0;
        let mut opos = 0;
        let input_size = input.len();
        let output_size = output.len();

        while ipos + sample_size <= input_size
            && (opos + sample_size / 2 * self.channels.len() * 4 <= output_size
                || self.decode_skip_leading > 0)
        {
            if self.decode_samples(&input[ipos..]) != 0 {
                break;
            }
            if self.decode_skip_leading > 0 {
                self.decode_skip_leading -= 1;
                if self.decode_skip_leading > 0 {
                    ipos += sample_size;
                    continue;
                }
            }
            for sample in 0..4 {
                for channel in self.channels.iter_mut() {
                    if self.hd {
                        output[opos] = channel.samples[sample] as u8;
                        output[opos + 1] = (channel.samples[sample] >> 8) as u8;
                        output[opos + 2] = (channel.samples[sample] >> 16) as u8;
                    } else {
                        output[opos] = (channel.samples[sample] >> 8) as u8;
                        output[opos + 1] = (channel.samples[sample] >> 16) as u8;
                    }
                    opos += sample_size / 2;
                }
            }
            ipos += sample_size;
        }

        *written = opos;
        ipos
    }

    pub fn decode_sync(
        &mut self,
        input: &[u8],
        output: &mut [u8],
        written: &mut usize,
        synced: &mut bool,
        dropped: &mut usize,
    ) -> usize {
        let sample_size = if self.hd { 6 } else { 4 };
        let mut ipos = 0;
        let mut opos = 0;
        let output_size = output.len();
        let input_size = input.len();

        *synced = false;
        *dropped = 0;

        if self.decode_sync_buffer_len > 0
            && sample_size - 1 - self.decode_sync_buffer_len as usize <= input_size
        {
            while self.decode_sync_buffer_len < sample_size as u8 - 1 {
                self.decode_sync_buffer[self.decode_sync_buffer_len as usize] = input[ipos];
                ipos += 1;
                self.decode_sync_buffer_len += 1;
            }
        }

        while self.decode_sync_buffer_len == sample_size as u8 - 1
            && ipos < sample_size
            && ipos < input_size
            && (opos + sample_size / 2 * self.channels.len() * 4 <= output_size
                || self.decode_skip_leading > 0
                || self.decode_dropped > 0)
        {
            self.decode_sync_buffer[sample_size - 1] = input[ipos];
            ipos += 1;

            let decode_sync_buffer_copy = self.decode_sync_buffer[..sample_size].to_vec();
            let processed_step =
                self.decode(&decode_sync_buffer_copy, &mut output[opos..], written);
            self.decode_sync_buffer[..sample_size].copy_from_slice(&decode_sync_buffer_copy);

            opos += *written;

            if self.decode_dropped > 0 && processed_step == sample_size {
                self.decode_dropped += processed_step;
                self.decode_sync_packets += 1;
                if self.decode_sync_packets >= (Self::LATENCY_SAMPLES + sample_size / 2) / 4 {
                    *dropped += self.decode_dropped;
                    self.decode_dropped = 0;
                    self.decode_sync_packets = 0;
                }
            }

            if processed_step < sample_size {
                self.reset_decode_sync();
                *synced = false;
                self.decode_dropped += 1;
                self.decode_sync_packets = 0;
                for i in 0..sample_size - 1 {
                    self.decode_sync_buffer[i] = self.decode_sync_buffer[i + 1];
                }
            } else {
                if self.decode_dropped == 0 {
                    *synced = true;
                }
                self.decode_sync_buffer_len = 0;
            }
        }

        if self.decode_sync_buffer_len == sample_size as u8 - 1 && ipos == sample_size {
            ipos = 0;
            self.decode_sync_buffer_len = 0;
        }

        while ipos + sample_size <= input_size
            && (opos + sample_size / 2 * self.channels.len() * 4 <= output_size
                || self.decode_skip_leading > 0
                || self.decode_dropped > 0)
        {
            let mut input_size_step = ((output_size - opos)
                / (sample_size / 2 * self.channels.len() * 4)
                + self.decode_skip_leading as usize)
                * sample_size;
            if input_size_step > ((input_size - ipos) / sample_size) * sample_size {
                input_size_step = ((input_size - ipos) / sample_size) * sample_size;
            }
            if input_size_step
                > ((Self::LATENCY_SAMPLES + sample_size / 2) / 4 - self.decode_sync_packets)
                    * sample_size
                && self.decode_dropped > 0
            {
                input_size_step = ((Self::LATENCY_SAMPLES + sample_size / 2) / 4
                    - self.decode_sync_packets)
                    * sample_size;
            }
            let input_size = input_size_step.min((input_size - ipos) / sample_size * sample_size);
            let processed_step = self.decode(
                &input[ipos..(ipos + input_size)],
                &mut output[opos..],
                written,
            );

            ipos += processed_step;
            opos += *written;

            if self.decode_dropped > 0 && processed_step / sample_size > 0 {
                self.decode_dropped += processed_step;
                self.decode_sync_packets += processed_step / sample_size;
                if self.decode_sync_packets >= (Self::LATENCY_SAMPLES + sample_size / 2) / 4 {
                    *dropped += self.decode_dropped;
                    self.decode_dropped = 0;
                    self.decode_sync_packets = 0;
                }
            }

            if processed_step < input_size_step {
                self.reset_decode_sync();
                *synced = false;
                ipos += 1;
                self.decode_dropped += 1;
                self.decode_sync_packets = 0;
            } else if self.decode_dropped == 0 {
                *synced = true;
            }
        }

        if ipos + sample_size > input_size {
            while ipos < input_size {
                self.decode_sync_buffer[self.decode_sync_buffer_len as usize] = input[ipos];
                self.decode_sync_buffer_len += 1;
                ipos += 1;
            }
        }

        *written = opos;
        ipos
    }

    pub fn decode_sync_finish(&mut self) -> usize {
        let dropped = self.decode_sync_buffer_len as usize;
        self.reset();
        dropped
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clip_intp2() {
        assert_eq!(clip_intp2(1000, 10), 1000);
        assert_eq!(clip_intp2(1 << 11, 10), (1 << 10) - 1);
        assert_eq!(clip_intp2(-(1 << 11), 10), -(1 << 10));
    }

    #[test]
    fn test_clip() {
        assert_eq!(10.clamp(0, 100), 10);
        assert_eq!((-10).clamp(0, 100), 0);
        assert_eq!(200.clamp(0, 100), 100);
    }

    #[test]
    fn test_sign_extend() {
        assert_eq!(sign_extend(0b01111111, 7), -1);
        assert_eq!(sign_extend(0b10000000, 7), 0);
        assert_eq!(sign_extend(0b11111111, 7), -1);
    }

    #[test]
    fn test_rshift32() {
        assert_eq!(rshift32(1000, 1), 500);
        assert_eq!(rshift32(1001, 1), 500);
        assert_eq!(rshift32(-1000, 1), -500);
    }

    #[test]
    fn test_rshift64() {
        assert_eq!(rshift64(10000000000, 1), 5000000000);
        assert_eq!(rshift64(10000000001, 1), 5000000000);
        assert_eq!(rshift64(-10000000000, 1), -5000000000);
    }

    #[test]
    fn test_aptx_update_codeword_history() {
        let mut channel = AptxChannel::default();
        channel.quantize[0].quantized_sample = 1;
        channel.quantize[1].quantized_sample = 2;
        channel.quantize[2].quantized_sample = 3;
        channel.update_codeword_history();
        assert_eq!(channel.codeword_history, 3328);
    }

    #[test]
    fn test_aptx_generate_dither() {
        let mut channel = AptxChannel {
            codeword_history: 12345,
            ..Default::default()
        };
        channel.generate_dither();
        assert_eq!(channel.dither[0], -209715200);
    }

    #[test]
    fn test_aptx_qmf_filter_signal_push() {
        let mut signal = AptxFilterSignal::default();
        signal.qmf_filter_signal_push(123);
        assert_eq!(signal.buffer[0], 123);
        assert_eq!(signal.buffer[AptxFilterSignal::FILTER_TAPS], 123);
        assert_eq!(signal.pos, 1);
    }

    #[test]
    fn test_aptx_qmf_convolution() {
        let mut signal = AptxFilterSignal::default();
        for i in 0..AptxFilterSignal::FILTER_TAPS {
            signal.qmf_filter_signal_push(i as i32);
        }
        let coeffs = [1; AptxFilterSignal::FILTER_TAPS];
        let result = signal.qmf_convolution(&coeffs, 5);
        assert_eq!(result, 4);
    }

    #[test]
    fn test_aptx_bin_search() {
        let intervals = [0, 10, 20, 30, 40, 50, 60, 70];
        let factor = 2;
        let value = 15;
        let result = aptx_bin_search(value, factor, &intervals);
        assert_eq!(result, 7);
    }

    #[test]
    fn test_aptx_quantize_difference() {
        let mut quantize = AptxQuantize::default();
        let sample_difference = 500;
        let dither = 100;
        let quantization_factor = 2048;
        let tables = &AptxChannel::ALL_TABLES[0][0];
        quantize.quantize_difference(sample_difference, dither, quantization_factor, tables);
        assert_eq!(quantize.quantized_sample, 12);
    }

    #[test]
    fn test_aptx_encode_channel() {
        let mut channel = AptxChannel {
            samples: [1000, 2000, 3000, 4000],
            ..Default::default()
        };
        channel.encode_channel(false);
        assert_eq!(channel.quantize[0].quantized_sample, 63);
    }

    #[test]
    fn test_aptx_decode_channel() {
        let mut channel = AptxChannel::default();
        channel.prediction[0].previous_reconstructed_sample = 1000;
        channel.decode_channel();
        assert_eq!(channel.samples[0], 0);
    }

    #[test]
    fn test_aptx_pack_codeword() {
        let mut channel = AptxChannel {
            quantize: [
                AptxQuantize {
                    quantized_sample: 1,
                    ..Default::default()
                },
                AptxQuantize {
                    quantized_sample: 2,
                    ..Default::default()
                },
                AptxQuantize {
                    quantized_sample: 3,
                    ..Default::default()
                },
                AptxQuantize {
                    quantized_sample: 4,
                    ..Default::default()
                },
            ],
            ..Default::default()
        };
        let result = channel.pack_codeword();
        assert_eq!(result, 39169);
    }

    #[test]
    fn test_aptx_unpack_codeword() {
        let mut channel = AptxChannel::default();
        let codeword = 0b0010100010010001;
        channel.unpack_codeword(codeword);
        assert_eq!(channel.quantize[0].quantized_sample, 17);
        assert_eq!(channel.quantize[1].quantized_sample, 1);
        assert_eq!(channel.quantize[2].quantized_sample, 1);
        assert_eq!(channel.quantize[3].quantized_sample, 0);
    }

    #[test]
    fn test_aptx_init() {
        let ctx = AptxContext::new(false);
        assert!(!ctx.hd);
        assert_eq!(
            ctx.decode_skip_leading,
            ((AptxContext::LATENCY_SAMPLES + 3) / 4) as u8
        );
        assert_eq!(
            ctx.encode_remaining,
            ((AptxContext::LATENCY_SAMPLES + 3) / 4) as u8
        );
    }

    #[test]
    fn test_aptx_reset() {
        let mut ctx = AptxContext::new(false);
        ctx.reset();
        assert!(!ctx.hd);
        assert_eq!(
            ctx.decode_skip_leading,
            ((AptxContext::LATENCY_SAMPLES + 3) / 4) as u8
        );
        assert_eq!(
            ctx.encode_remaining,
            ((AptxContext::LATENCY_SAMPLES + 3) / 4) as u8
        );
    }

    #[test]
    fn test_aptx_encode() {
        let mut ctx = AptxContext::new(false);
        let input = vec![0; 16];
        let mut output = vec![0; 16];
        let mut written = 0;
        let read = ctx.encode(&input, &mut output, &mut written);
        assert_eq!(read, input.len());
        assert!(written > 0);
    }

    #[test]
    fn test_aptx_decode() {
        let mut ctx = AptxContext::new(false);
        let input = vec![0; 16];
        let mut output = vec![0; 16];
        let mut written = 0;
        let read = ctx.decode(&input, &mut output, &mut written);
        assert_eq!(read, 16);
    }

    #[test]
    fn test_aptx_encode_finish() {
        let mut ctx = AptxContext::new(false);
        let mut output = vec![0; 16];
        let mut written = 0;
        let result = ctx.encode_finish(&mut output, &mut written);
        assert_eq!(result, 0);
        assert!(written > 0);
    }

    #[test]
    fn test_aptx_decode_sync() {
        let mut ctx = AptxContext::new(false);
        let input = vec![0; 16];
        let mut output = vec![0; 16];
        let mut written = 0;
        let mut synced = false;
        let mut dropped = 0;
        let read = ctx.decode_sync(&input, &mut output, &mut written, &mut synced, &mut dropped);
        assert_eq!(read, input.len());
        assert!(written == 0);
        assert_eq!(synced, true);
        assert_eq!(dropped, 0);
    }

    #[test]
    fn test_aptx_decode_sync_finish() {
        let mut ctx = AptxContext::new(false);
        ctx.decode_sync_buffer_len = 5;
        let dropped = ctx.decode_sync_finish();
        assert_eq!(dropped, 5);
    }
}
