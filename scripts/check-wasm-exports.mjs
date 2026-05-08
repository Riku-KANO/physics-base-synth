import { readFileSync } from 'node:fs';
import { resolve, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';

const __dirname = dirname(fileURLToPath(import.meta.url));
const wasmPath = resolve(__dirname, '../web/src/lib/wasm/wasm_audio.wasm');

const REQUIRED = [
  'memory',
  'synth_new',
  'synth_free',
  'synth_note_on',
  'synth_note_off',
  'synth_set_param',
  'synth_reset',
  'synth_out_l_ptr',
  'synth_out_r_ptr',
  'synth_capacity',
  'synth_process_block',
  'synth_set_polyphony_mode',
  // Phase 3 追加 (D38 / D39 / D41)
  'synth_midi_cc',
  'synth_pitch_bend',
  'synth_voice_state_ptr',
  // Phase 4a 追加 (D45-D52)
  'synth_apply_instrument',
  'synth_lfo_set_rate',
  'synth_lfo_set_waveform',
  'synth_lfo_set_depth',
];

const bytes = readFileSync(wasmPath);
const mod = await WebAssembly.compile(bytes);
const exports = WebAssembly.Module.exports(mod).map((e) => e.name);
const missing = REQUIRED.filter((n) => !exports.includes(n));
if (missing.length) {
  console.error('Missing WASM exports:', missing);
  console.error('Available:', exports);
  process.exit(1);
}
console.log('All required WASM exports present.');
