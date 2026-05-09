// web/src/lib/audio/__synthDev.ts
//
// Phase 4b D66: F38b 計測自動化スクリプト (dev only)。
//
// 使い方:
//   await window.__synthDev.measureProcessTime(10000);
//   // → { avg: 0.045, max: 0.087, samples: [...], bufferOverflow: false }
//
// production build では `--define:DEV_MODE=false` を渡すことで Worklet 側の
// timing 集約コードが完全に tree-shake される。Web 側 (本ファイル) も
// `if (import.meta.env.DEV)` ガードで synth.svelte.ts から動的 import する。
//
// 計測方式: Worklet 側で performance.now() の差分を取り、リングバッファ (4096 entry)
// に self time (ms) を蓄積。stop メッセージで時系列順に main へ送る。
// 4096 entry は 48kHz/128 frames で約 10.92 秒分、durationMs > 10000 では古い
// サンプルが上書きされ最新 ~10.92 秒分が保持される (bufferOverflow=true で報告)。

import type { ToWorkletMessage, FromWorkletMessage } from './messages';

export interface TimingResult {
	avg: number;
	max: number;
	min: number;
	samples: number[];
	bufferOverflow: boolean;
	durationMs: number;
}

/**
 * AudioWorklet `process` の self time を `durationMs` ms 間計測し、
 * avg / max / min を返す。
 *
 * Worklet 側でリングバッファ (4096 entry × f32 = 16 KB) に self time を蓄積、
 * stop メッセージで時系列順の有効サンプルを main へ集約 postMessage。
 * 4096 entry は約 10.92 秒分 (48kHz / 128 frames quanta) を保持、durationMs > 10000
 * では bufferOverflow=true で報告される (最新 ~10.92 秒分のみ有効)。
 */
export async function measureProcessTime(
	port: MessagePort,
	durationMs: number
): Promise<TimingResult> {
	return new Promise((resolve, reject) => {
		const timeoutId = setTimeout(() => {
			port.removeEventListener('message', onMessage);
			reject(new Error(`measureProcessTime timeout (${durationMs}ms)`));
		}, durationMs + 5000);

		function onMessage(e: MessageEvent<FromWorkletMessage>) {
			if (e.data.type !== 'timing') return;
			port.removeEventListener('message', onMessage);
			clearTimeout(timeoutId);

			const samples = e.data.samples;
			if (samples.length === 0) {
				resolve({
					avg: 0,
					max: 0,
					min: 0,
					samples: [],
					bufferOverflow: e.data.bufferOverflow,
					durationMs
				});
				return;
			}

			let sum = 0;
			let max = -Infinity;
			let min = Infinity;
			for (const s of samples) {
				sum += s;
				if (s > max) max = s;
				if (s < min) min = s;
			}
			resolve({
				avg: sum / samples.length,
				max,
				min,
				samples,
				bufferOverflow: e.data.bufferOverflow,
				durationMs
			});
		}

		port.addEventListener('message', onMessage);

		// Phase 4a の onmessage は SynthEngine が握っているため、addEventListener 経由で
		// 共存させる (重複消費にはならない: addEventListener と onmessage は両方発火する
		// が、port.start() が必要)。SynthEngine 側で port.onmessage を設定済のため
		// start() は呼ばれている前提。
		port.postMessage({ type: 'startTimingCapture' } as ToWorkletMessage);

		setTimeout(() => {
			port.postMessage({ type: 'stopTimingCapture' } as ToWorkletMessage);
		}, durationMs);
	});
}
