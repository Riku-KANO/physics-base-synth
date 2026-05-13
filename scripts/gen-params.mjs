import { readFileSync, writeFileSync, mkdirSync } from 'node:fs';
import { resolve, dirname } from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';

const __dirname = dirname(fileURLToPath(import.meta.url));
const root = resolve(__dirname, '..');

function formatF32(value) {
	if (!Number.isFinite(value)) {
		throw new Error(`non-finite f32 literal: ${value}`);
	}
	// JS double で計算した値は f32 では過剰精度になる (e.g. 0.4 * 1.05 = 0.42000000000000004)。
	// Math.fround で f32 に丸めてから 7 桁有効数字で再文字列化、Rust の clippy::excessive_precision を回避。
	const f32 = Math.fround(value);
	const trimmed = Number(f32.toPrecision(7));
	if (Number.isInteger(trimmed)) {
		return `${trimmed}.0`;
	}
	return String(trimmed);
}

function formatTsNumber(value) {
	if (!Number.isFinite(value)) {
		throw new Error(`non-finite number literal: ${value}`);
	}
	return String(value);
}

function constName(name) {
	return name
		.replace(/([a-z0-9])([A-Z])/g, '$1_$2')
		.replace(/([A-Z]+)([A-Z][a-z])/g, '$1_$2')
		.toUpperCase();
}

const NAME_RE = /^[A-Z][A-Za-z0-9]*$/;

function validateParams(params) {
	if (!Array.isArray(params)) {
		throw new Error('params must be an array');
	}
	for (let i = 0; i < params.length; i++) {
		const p = params[i];
		if (p.id !== i) {
			throw new Error(`params[${i}].id must equal ${i}, got ${p.id}`);
		}
		if (typeof p.name !== 'string' || !NAME_RE.test(p.name)) {
			throw new Error(
				`params[${i}].name must match /^[A-Z][A-Za-z0-9]*$/ (used as Rust enum variant + JS object key), got ${JSON.stringify(p.name)}`
			);
		}
		for (const k of ['min', 'max', 'default', 'smoothing_tau']) {
			if (typeof p[k] !== 'number' || !Number.isFinite(p[k])) {
				throw new Error(`params[${i}].${k} must be a finite number, got ${p[k]}`);
			}
		}
		if (p.min > p.max) {
			throw new Error(`params[${i}] (${p.name}): min (${p.min}) > max (${p.max})`);
		}
		if (p.default < p.min || p.default > p.max) {
			throw new Error(
				`params[${i}] (${p.name}): default (${p.default}) outside [min=${p.min}, max=${p.max}]`
			);
		}
		if (p.smoothing_tau <= 0) {
			throw new Error(`params[${i}] (${p.name}): smoothing_tau must be > 0, got ${p.smoothing_tau}`);
		}
	}
}

function validateBodyModes(modes) {
	if (!Array.isArray(modes)) {
		throw new Error('body_modes must be an array');
	}
	// Phase 4c R44 緩和策 2 (Step 15): Piano は M=16 modes に拡張可能。
	// 他楽器は Phase 4a/4b 互換性のため 8 modes 固定。
	if (modes.length !== 8 && modes.length !== 16) {
		throw new Error(`body_modes must have 8 or 16 entries, got ${modes.length}`);
	}
	for (let i = 0; i < modes.length; i++) {
		const m = modes[i];
		for (const k of ['freq', 'q', 'gain']) {
			if (typeof m[k] !== 'number' || !Number.isFinite(m[k]) || m[k] <= 0) {
				throw new Error(`body_modes[${i}].${k} must be a positive finite number, got ${m[k]}`);
			}
		}
	}
}

function validateInstruments(instruments) {
	if (!Array.isArray(instruments)) {
		throw new Error('instruments must be an array');
	}
	if (instruments.length === 0) {
		throw new Error('instruments must contain at least one entry');
	}
	if (instruments[0].kind !== 'Default') {
		throw new Error(
			`instruments[0].kind must be "Default" (Phase 3 互換 / kind=0), got ${JSON.stringify(instruments[0].kind)}`
		);
	}
	const seen = new Set();
	for (let i = 0; i < instruments.length; i++) {
		const ins = instruments[i];
		if (typeof ins.kind !== 'string' || !NAME_RE.test(ins.kind)) {
			throw new Error(
				`instruments[${i}].kind must match /^[A-Z][A-Za-z0-9]*$/, got ${JSON.stringify(ins.kind)}`
			);
		}
		if (seen.has(ins.kind)) {
			throw new Error(`instruments[${i}].kind duplicates earlier entry: ${ins.kind}`);
		}
		seen.add(ins.kind);
		if (
			typeof ins.stereo_spread !== 'number' ||
			!Number.isFinite(ins.stereo_spread) ||
			ins.stereo_spread < 0
		) {
			throw new Error(
				`instruments[${i}] (${ins.kind}): stereo_spread must be a non-negative finite number, got ${ins.stereo_spread}`
			);
		}
		validateBodyModes(ins.body_modes);
		// Phase 4b D62: Piano kind は inharmonicity_b / hammer_cutoff_low_hz / hammer_cutoff_high_hz を必須とする。
		// Default〜Sitar (0-6) にこれらのフィールドが付与されるのは将来の Phase 4c で複数 Piano 機種が
		// 出てきた場合を想定するが、Phase 4b では Piano kind 限定で扱い、付与されていれば検証して通す。
		if (ins.kind === 'Piano') {
			if (typeof ins.inharmonicity_b !== 'number' || !Number.isFinite(ins.inharmonicity_b) || ins.inharmonicity_b <= 0) {
				throw new Error(
					`instruments[${i}] (Piano): inharmonicity_b must be a positive finite number, got ${ins.inharmonicity_b}`
				);
			}
			if (typeof ins.hammer_cutoff_low_hz !== 'number' || !Number.isFinite(ins.hammer_cutoff_low_hz) || ins.hammer_cutoff_low_hz <= 0) {
				throw new Error(
					`instruments[${i}] (Piano): hammer_cutoff_low_hz must be a positive finite number, got ${ins.hammer_cutoff_low_hz}`
				);
			}
			if (typeof ins.hammer_cutoff_high_hz !== 'number' || !Number.isFinite(ins.hammer_cutoff_high_hz) || ins.hammer_cutoff_high_hz <= ins.hammer_cutoff_low_hz) {
				throw new Error(
					`instruments[${i}] (Piano): hammer_cutoff_high_hz must be greater than hammer_cutoff_low_hz, got high=${ins.hammer_cutoff_high_hz}, low=${ins.hammer_cutoff_low_hz}`
				);
			}
			// Phase 4c D72 / D77 / D78: Piano-only fields driven by params.json.
			if (typeof ins.unison_detune_cents !== 'number' || !Number.isFinite(ins.unison_detune_cents) || ins.unison_detune_cents < 0) {
				throw new Error(
					`instruments[${i}] (Piano): unison_detune_cents must be a non-negative finite number, got ${ins.unison_detune_cents}`
				);
			}
			if (typeof ins.sympathetic_amount !== 'number' || !Number.isFinite(ins.sympathetic_amount) || ins.sympathetic_amount < 0 || ins.sympathetic_amount > 1.0) {
				throw new Error(
					`instruments[${i}] (Piano): sympathetic_amount must be in [0.0, 1.0], got ${ins.sympathetic_amount}`
				);
			}
			if (!Array.isArray(ins.inharmonicity_b_curve) || ins.inharmonicity_b_curve.length !== 88) {
				throw new Error(
					`instruments[${i}] (Piano): inharmonicity_b_curve must be an array of length 88 (A0=21..C8=108), got length ${Array.isArray(ins.inharmonicity_b_curve) ? ins.inharmonicity_b_curve.length : typeof ins.inharmonicity_b_curve}`
				);
			}
			for (let k = 0; k < ins.inharmonicity_b_curve.length; k++) {
				const value = ins.inharmonicity_b_curve[k];
				if (typeof value !== 'number' || !Number.isFinite(value) || value <= 0) {
					throw new Error(
						`instruments[${i}] (Piano): inharmonicity_b_curve[${k}] must be a positive finite number, got ${value}`
					);
				}
			}
		}
	}
}

/**
 * Phase 3 D32: 左右 ch で freq / q を ±spread% 揺らす純粋関数。
 * 偶数 index は freq +spread / q -spread、奇数 index は freq -spread / q +spread で
 * 反転させ、左右の chorus 的広がりを生成。gain は全モード一律 +spread%。
 */
export function applyStereoSpread(modes, spread) {
	return modes.map((m, i) => ({
		freq: m.freq * (i % 2 === 0 ? 1 + spread : 1 - spread),
		q: m.q * (i % 2 === 0 ? 1 - spread : 1 + spread),
		gain: m.gain * (1 + spread)
	}));
}

export function generateRustSource(paramsJson) {
	const params = paramsJson.params;
	validateParams(params);
	const instruments = paramsJson.instruments ?? null;
	if (instruments !== null) {
		validateInstruments(instruments);
	}

	const lines = [];
	lines.push('// AUTO-GENERATED FROM params.json — DO NOT EDIT');
	lines.push('// Run `pnpm gen:params` to regenerate.');
	lines.push('');
	// Phase 4a: 楽器係数 (Modal gain など) が偶然 π / 1/π / 1/√2 等の数学定数に
	// 近い値になることがあるため、approx_constant の clippy lint を抑止する。
	lines.push('#![allow(clippy::approx_constant)]');
	lines.push('');
	lines.push('#[derive(Debug, Clone, Copy)]');
	lines.push('pub struct ParamDescriptor {');
	lines.push('    pub id: u32,');
	lines.push("    pub name: &'static str,");
	lines.push('    pub min: f32,');
	lines.push('    pub max: f32,');
	lines.push('    pub default: f32,');
	lines.push('    pub smoothing_tau: f32,');
	lines.push('}');
	lines.push('');
	lines.push('impl ParamDescriptor {');
	lines.push('    pub const fn clamp(&self, value: f32) -> f32 {');
	lines.push('        if value < self.min {');
	lines.push('            self.min');
	lines.push('        } else if value > self.max {');
	lines.push('            self.max');
	lines.push('        } else {');
	lines.push('            value');
	lines.push('        }');
	lines.push('    }');
	lines.push('}');
	lines.push('');
	lines.push('#[repr(u32)]');
	lines.push('#[non_exhaustive]');
	lines.push('#[derive(Debug, Copy, Clone, Eq, PartialEq)]');
	lines.push('pub enum ParamId {');
	for (const p of params) {
		lines.push(`    ${p.name} = ${p.id},`);
	}
	lines.push('}');
	lines.push('');
	lines.push('impl ParamId {');
	lines.push('    pub fn from_u32(value: u32) -> Option<Self> {');
	lines.push('        match value {');
	for (const p of params) {
		lines.push(`            ${p.id} => Some(Self::${p.name}),`);
	}
	lines.push('            _ => None,');
	lines.push('        }');
	lines.push('    }');
	lines.push('');
	lines.push("    pub fn descriptor(&self) -> &'static ParamDescriptor {");
	lines.push('        &PARAM_DESCRIPTORS[*self as usize]');
	lines.push('    }');
	lines.push('}');
	lines.push('');

	for (const p of params) {
		const descName = `${constName(p.name)}_DESCRIPTOR`;
		lines.push(`pub const ${descName}: ParamDescriptor = ParamDescriptor {`);
		lines.push(`    id: ${p.id},`);
		lines.push(`    name: "${p.name}",`);
		lines.push(`    min: ${formatF32(p.min)},`);
		lines.push(`    max: ${formatF32(p.max)},`);
		lines.push(`    default: ${formatF32(p.default)},`);
		lines.push(`    smoothing_tau: ${formatF32(p.smoothing_tau)},`);
		lines.push('};');
		lines.push('');
	}

	lines.push(`pub const PARAM_DESCRIPTORS: [ParamDescriptor; ${params.length}] = [`);
	for (const p of params) {
		lines.push(`    ${constName(p.name)}_DESCRIPTOR,`);
	}
	lines.push('];');
	lines.push('');

	lines.push('// Phase 1 互換の範囲定数（既存コードからの参照のため維持）');
	for (const p of params) {
		const base = constName(p.name);
		const descName = `${base}_DESCRIPTOR`;
		lines.push(`pub const ${base}_MIN: f32 = ${descName}.min;`);
		lines.push(`pub const ${base}_MAX: f32 = ${descName}.max;`);
		lines.push(`pub const ${base}_DEFAULT: f32 = ${descName}.default;`);
		lines.push('');
	}

	if (instruments !== null) {
		// Phase 3 D30 / D32 + Phase 4a D52 / D54: ModalBodyResonator の係数テーブル
		lines.push('// Phase 3 D30 / D32 + Phase 4a D52 / D54: ModalBodyResonator の係数テーブル');
		lines.push('#[derive(Debug, Clone, Copy)]');
		lines.push('pub struct BodyMode {');
		lines.push('    pub freq: f32,');
		lines.push('    pub q: f32,');
		lines.push('    pub gain: f32,');
		lines.push('}');
		lines.push('');

		// Phase 4a D54: 楽器ごとの STEREO_SPREAD
		for (const ins of instruments) {
			const upper = constName(ins.kind);
			lines.push(`pub const STEREO_SPREAD_${upper}: f32 = ${formatF32(ins.stereo_spread)};`);
		}
		lines.push('');

		// Phase 4a D52 + Phase 4c R44 緩和策 2: 楽器ごとの BODY_MODES_<INSTRUMENT>_L/R
		// (Default 等 = 8 modes、Piano = 16 modes)。
		for (const ins of instruments) {
			const upper = constName(ins.kind);
			const modesL = ins.body_modes;
			const modesR = applyStereoSpread(modesL, ins.stereo_spread);

			lines.push('#[rustfmt::skip]');
			lines.push(`pub const BODY_MODES_${upper}_L: [BodyMode; ${modesL.length}] = [`);
			for (const m of modesL) {
				lines.push(
					`    BodyMode { freq: ${formatF32(m.freq)}, q: ${formatF32(m.q)}, gain: ${formatF32(m.gain)} },`
				);
			}
			lines.push('];');
			lines.push('');

			lines.push('#[rustfmt::skip]');
			lines.push(`pub const BODY_MODES_${upper}_R: [BodyMode; ${modesR.length}] = [`);
			for (const m of modesR) {
				lines.push(
					`    BodyMode { freq: ${formatF32(m.freq)}, q: ${formatF32(m.q)}, gain: ${formatF32(m.gain)} },`
				);
			}
			lines.push('];');
			lines.push('');
		}

		// Phase 3 互換 alias: Default kind の係数を旧名で再 export
		lines.push('// Phase 3 互換: Default kind の alias');
		lines.push('pub const BODY_MODES_L: [BodyMode; 8] = BODY_MODES_DEFAULT_L;');
		lines.push('pub const BODY_MODES_R: [BodyMode; 8] = BODY_MODES_DEFAULT_R;');
		lines.push('pub const STEREO_SPREAD: f32 = STEREO_SPREAD_DEFAULT;');
		lines.push('');

		// Phase 4a D52: InstrumentKind enum
		lines.push('#[repr(u32)]');
		lines.push('#[non_exhaustive]');
		lines.push('#[derive(Debug, Copy, Clone, Eq, PartialEq)]');
		lines.push('pub enum InstrumentKind {');
		for (let i = 0; i < instruments.length; i++) {
			lines.push(`    ${instruments[i].kind} = ${i},`);
		}
		lines.push('}');
		lines.push('');
		lines.push('impl InstrumentKind {');
		lines.push('    pub fn from_u32(value: u32) -> Option<Self> {');
		lines.push('        match value {');
		for (let i = 0; i < instruments.length; i++) {
			lines.push(`            ${i} => Some(Self::${instruments[i].kind}),`);
		}
		lines.push('            _ => None,');
		lines.push('        }');
		lines.push('    }');
		lines.push('}');
		lines.push('');
		lines.push(`pub const INSTRUMENT_KIND_COUNT: usize = ${instruments.length};`);
		lines.push('');

		// Phase 4b D58 / D61 / D62 + Phase 4c D72 / D75 / D77 / D78: Piano 専用フィールド
		// (Piano kind のみ持つ)。Phase 4b では Piano 1 機種なので const として出力、
		// 複数 Piano 機種を扱う Phase 4d で楽器ごとの固有値を保持する設計に切り替える想定。
		const piano = instruments.find((i) => i.kind === 'Piano');
		if (piano) {
			lines.push(`pub const INHARMONICITY_B_PIANO: f32 = ${formatF32(piano.inharmonicity_b)};`);
			lines.push(`pub const HAMMER_CUTOFF_LOW_PIANO: f32 = ${formatF32(piano.hammer_cutoff_low_hz)};`);
			lines.push(`pub const HAMMER_CUTOFF_HIGH_PIANO: f32 = ${formatF32(piano.hammer_cutoff_high_hz)};`);
			lines.push(`pub const UNISON_DETUNE_CENTS_PIANO: f32 = ${formatF32(piano.unison_detune_cents)};`);
			lines.push(`pub const SYMPATHETIC_AMOUNT_PIANO: f32 = ${formatF32(piano.sympathetic_amount)};`);
			lines.push('');
			// Phase 4c D78 / D79: 88 鍵 × f32 LUT (A0=21..C8=108)。
			// `dispersion::b_curve_piano(midi)` が `midi.clamp(21, 108) - 21` を index に引く。
			lines.push('#[rustfmt::skip]');
			lines.push(`pub const INHARMONICITY_B_CURVE_PIANO: [f32; 88] = [`);
			for (let row = 0; row < 11; row++) {
				const chunk = piano.inharmonicity_b_curve.slice(row * 8, row * 8 + 8);
				lines.push('    ' + chunk.map((v) => formatF32(v)).join(', ') + ',');
			}
			lines.push('];');
			lines.push('');
		}

		// Phase 4a D52 / D54: ヘルパ関数
		// 1 行で 100 chars 超の match arm (例: GuitarClassical / GuitarSteel) は rustfmt が
		// 4 行に展開し、短い arm は 1 行で残すため、単純な generator 出力では行ごとに
		// 整形差が出る。関数全体に `#[rustfmt::skip]` を付けて生成側の出力をそのまま固定する
		// (Phase 1-3 の BODY_MODES_L と同パターン)。
		// Phase 4c R44 緩和策 2: slice 戻り値で楽器ごとの可変長 (Default 等 = 8、Piano = 16) を吸収。
		lines.push('#[rustfmt::skip]');
		lines.push('pub fn body_modes_for_instrument(');
		lines.push('    kind: InstrumentKind,');
		lines.push(") -> (&'static [BodyMode], &'static [BodyMode]) {");
		lines.push('    match kind {');
		for (const ins of instruments) {
			const upper = constName(ins.kind);
			lines.push(
				`        InstrumentKind::${ins.kind} => (&BODY_MODES_${upper}_L, &BODY_MODES_${upper}_R),`
			);
		}
		lines.push('    }');
		lines.push('}');
		lines.push('');
		lines.push('pub fn stereo_spread_for_instrument(kind: InstrumentKind) -> f32 {');
		lines.push('    match kind {');
		for (const ins of instruments) {
			const upper = constName(ins.kind);
			lines.push(`        InstrumentKind::${ins.kind} => STEREO_SPREAD_${upper},`);
		}
		lines.push('    }');
		lines.push('}');
		lines.push('');
	}

	return lines.join('\n');
}

export function generateTsSource(paramsJson) {
	const params = paramsJson.params;
	validateParams(params);
	const instruments = paramsJson.instruments ?? null;
	if (instruments !== null) {
		validateInstruments(instruments);
	}

	const lines = [];
	lines.push('// AUTO-GENERATED FROM params.json — DO NOT EDIT');
	lines.push('// Run `pnpm gen:params` to regenerate.');
	lines.push('');
	lines.push('export interface ParamDescriptor {');
	lines.push('\treadonly id: number;');
	lines.push('\treadonly name: string;');
	lines.push('\treadonly min: number;');
	lines.push('\treadonly max: number;');
	lines.push('\treadonly default: number;');
	lines.push('\treadonly smoothingTau: number;');
	lines.push('}');
	lines.push('');
	lines.push('export const PARAM_IDS = {');
	for (let i = 0; i < params.length; i++) {
		const p = params[i];
		const sep = i < params.length - 1 ? ',' : '';
		lines.push(`\t${p.name}: ${p.id}${sep}`);
	}
	lines.push('} as const;');
	lines.push('');
	lines.push('export type ParamIdValue = (typeof PARAM_IDS)[keyof typeof PARAM_IDS];');
	lines.push('');
	lines.push('export const PARAM_DESCRIPTORS: readonly ParamDescriptor[] = [');
	for (let i = 0; i < params.length; i++) {
		const p = params[i];
		const sep = i < params.length - 1 ? ',' : '';
		lines.push(
			`\t{ id: ${p.id}, name: '${p.name}', min: ${formatTsNumber(p.min)}, max: ${formatTsNumber(p.max)}, default: ${formatTsNumber(p.default)}, smoothingTau: ${formatTsNumber(p.smoothing_tau)} }${sep}`
		);
	}
	lines.push('] as const;');
	lines.push('');
	lines.push('export function getDescriptor(id: ParamIdValue): ParamDescriptor {');
	lines.push('\treturn PARAM_DESCRIPTORS[id];');
	lines.push('}');
	lines.push('');
	lines.push('export function clampValue(id: ParamIdValue, value: number): number {');
	lines.push('\tconst d = PARAM_DESCRIPTORS[id];');
	lines.push('\treturn value < d.min ? d.min : value > d.max ? d.max : value;');
	lines.push('}');
	lines.push('');

	if (instruments !== null) {
		lines.push('// Phase 3 D30 / D32 + Phase 4a D52 / D54: ModalBodyResonator の係数テーブル');
		lines.push('export interface BodyMode {');
		lines.push('\treadonly freq: number;');
		lines.push('\treadonly q: number;');
		lines.push('\treadonly gain: number;');
		lines.push('}');
		lines.push('');

		// 楽器ごとの STEREO_SPREAD と BODY_MODES_<INSTRUMENT>_L/R
		for (const ins of instruments) {
			const upper = constName(ins.kind);
			const modesL = ins.body_modes;
			const modesR = applyStereoSpread(modesL, ins.stereo_spread);

			lines.push(`export const STEREO_SPREAD_${upper} = ${formatTsNumber(ins.stereo_spread)};`);
			lines.push('');
			lines.push(`export const BODY_MODES_${upper}_L: readonly BodyMode[] = [`);
			for (let i = 0; i < modesL.length; i++) {
				const m = modesL[i];
				const sep = i < modesL.length - 1 ? ',' : '';
				lines.push(
					`\t{ freq: ${formatTsNumber(m.freq)}, q: ${formatTsNumber(m.q)}, gain: ${formatTsNumber(m.gain)} }${sep}`
				);
			}
			lines.push('] as const;');
			lines.push('');
			lines.push(`export const BODY_MODES_${upper}_R: readonly BodyMode[] = [`);
			for (let i = 0; i < modesR.length; i++) {
				const m = modesR[i];
				const sep = i < modesR.length - 1 ? ',' : '';
				lines.push(
					`\t{ freq: ${formatTsNumber(m.freq)}, q: ${formatTsNumber(m.q)}, gain: ${formatTsNumber(m.gain)} }${sep}`
				);
			}
			lines.push('] as const;');
			lines.push('');
		}

		// Phase 3 互換 alias
		lines.push('// Phase 3 互換: Default kind の alias');
		lines.push('export const BODY_MODES_L = BODY_MODES_DEFAULT_L;');
		lines.push('export const BODY_MODES_R = BODY_MODES_DEFAULT_R;');
		lines.push('export const STEREO_SPREAD = STEREO_SPREAD_DEFAULT;');
		lines.push('');

		// Phase 4a D52: InstrumentKind enum
		lines.push('export const INSTRUMENT_KIND = {');
		for (let i = 0; i < instruments.length; i++) {
			const sep = i < instruments.length - 1 ? ',' : '';
			lines.push(`\t${instruments[i].kind}: ${i}${sep}`);
		}
		lines.push('} as const;');
		lines.push('');
		lines.push('export type InstrumentKindKey = keyof typeof INSTRUMENT_KIND;');
		lines.push('export type InstrumentKindValue = (typeof INSTRUMENT_KIND)[InstrumentKindKey];');
		lines.push('');
		lines.push(`export const INSTRUMENT_KIND_COUNT = ${instruments.length};`);
		lines.push('');

		// Phase 4b D58 / D61 / D62 + Phase 4c D72 / D75 / D77 / D78: Piano 専用フィールド
		// (TS 側にも出力、UI / preset で参照する用途、drift 防止)。UI 露出は Phase 4d 送り (D81)。
		const piano = instruments.find((i) => i.kind === 'Piano');
		if (piano) {
			lines.push(`export const INHARMONICITY_B_PIANO = ${formatTsNumber(piano.inharmonicity_b)};`);
			lines.push(`export const HAMMER_CUTOFF_LOW_PIANO = ${formatTsNumber(piano.hammer_cutoff_low_hz)};`);
			lines.push(`export const HAMMER_CUTOFF_HIGH_PIANO = ${formatTsNumber(piano.hammer_cutoff_high_hz)};`);
			lines.push(`export const UNISON_DETUNE_CENTS_PIANO = ${formatTsNumber(piano.unison_detune_cents)};`);
			lines.push(`export const SYMPATHETIC_AMOUNT_PIANO = ${formatTsNumber(piano.sympathetic_amount)};`);
			lines.push('');
			lines.push('export const INHARMONICITY_B_CURVE_PIANO: readonly number[] = [');
			for (let row = 0; row < 11; row++) {
				const chunk = piano.inharmonicity_b_curve.slice(row * 8, row * 8 + 8);
				const last = row === 10;
				lines.push('\t' + chunk.map((v) => formatTsNumber(v)).join(', ') + (last ? '' : ','));
			}
			lines.push('] as const;');
			lines.push('');
		}
	}

	return lines.join('\n');
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
	const paramsJson = JSON.parse(readFileSync(resolve(root, 'params.json'), 'utf8'));

	const rustPath = resolve(root, 'crates/dsp-core/src/params.rs');
	const tsPath = resolve(root, 'web/src/lib/audio/generated/params.ts');

	mkdirSync(dirname(rustPath), { recursive: true });
	mkdirSync(dirname(tsPath), { recursive: true });

	writeFileSync(rustPath, generateRustSource(paramsJson));
	writeFileSync(tsPath, generateTsSource(paramsJson));

	console.log(`Generated ${rustPath}`);
	console.log(`Generated ${tsPath}`);
}
