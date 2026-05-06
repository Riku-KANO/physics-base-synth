import { readFileSync, writeFileSync, mkdirSync } from 'node:fs';
import { resolve, dirname } from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';

const __dirname = dirname(fileURLToPath(import.meta.url));
const root = resolve(__dirname, '..');

function formatF32(value) {
	if (!Number.isFinite(value)) {
		throw new Error(`non-finite f32 literal: ${value}`);
	}
	if (Number.isInteger(value)) {
		return `${value}.0`;
	}
	return String(value);
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

export function generateRustSource(paramsJson) {
	const params = paramsJson.params;
	validateParams(params);

	const lines = [];
	lines.push('// AUTO-GENERATED FROM params.json — DO NOT EDIT');
	lines.push('// Run `pnpm gen:params` to regenerate.');
	lines.push('');
	lines.push('#[derive(Debug, Clone, Copy)]');
	lines.push('pub struct ParamDescriptor {');
	lines.push('    pub id: u32,');
	lines.push('    pub name: &\'static str,');
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
	lines.push('    pub fn descriptor(&self) -> &\'static ParamDescriptor {');
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

	return lines.join('\n');
}

export function generateTsSource(paramsJson) {
	const params = paramsJson.params;
	validateParams(params);

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
