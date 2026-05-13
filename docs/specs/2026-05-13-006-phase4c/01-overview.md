# 01. Phase 4c 概要とスコープ

## 目的

Phase 4b で確立した「ブラウザで動作する 8 音ポリフォニック Karplus–Strong + Modal Body Resonator + Extended KS + MIDI CC + Voice Meter UI + Soft clip + Thiran allpass + LFO + Mod Wheel + Preset (localStorage v1) + 多楽器プリセット 8 種 (Default + 7 楽器、Piano 含む) + Stretching all-pass cascade M=8 + Commuted impulse + velocity-dependent LPF + `__synthDev.measureProcessTime` + `.gitattributes` LF 統一」を土台に、**本格ピアノ音色を Multi-string per voice (1/2/3 弦、鍵盤位置依存) + Unison detuning (±1.5 cents) + Hertz law raised cosine hammer (接触時間 1.2〜4 ms、cutoff 800〜5500 Hz) + Global sympathetic resonance bus + 88 鍵 Inharmonicity B(note) LUT + Piano プリセット聴感チューニング**で実装し、Phase 4b retrospective §5 / §7 で明らかになった「Piano 音色が弦楽器寄り」という最大の構造的負債を解消する。

補助的に **Phase 4b 持ち越しの F38b 実機計測値取得** と **Phase 4a/4b 互換性の機械保証（`n_strings = 1` で Phase 4a HEAD と ε=1e-6 バイト一致継承）** を含める。Phase 1 / Phase 2 / Phase 3 / Phase 4a / Phase 4b の互換性制約（C ABI、リアルタイム制約、Svelte 5 runes、依存ゼロ、Mod Wheel = 0 で Phase 3 互換、Default kind で Phase 4a 互換）はすべて維持する。

## 他文書との関係

- 上流: [`pre-research.md`](./pre-research.md)（Phase 4c 追加調査、§2 Phase 4b 振り返り / §3 Multi-string / §4 Hertz hammer / §5 Sympathetic resonance / §6 B(note) LUT / §7 Modal Body M=16 / §8 補助候補 / §9 C8 自己発振 / §10 WASM SIMD / §11 性能予算）、[Phase 1〜4b 全 8 章](../)（既存資産）
- 下流: [`02-architecture.md`](./02-architecture.md)（全体構成の差分）→ `03〜05`（各レイヤ詳細）→ [`06-build-and-verify.md`](./06-build-and-verify.md) → [`07-implementation-checklist.md`](./07-implementation-checklist.md)
- 並行: `docs/retrospective/2026-05-09-005-phase4b.md`（Phase 4b 振り返り、§5 既存負債 / §7 推奨スコープを本フェーズで一部解消）
- 本書は「Phase 4c で何を作るか」を確定し、以降の文書は「どう作るか」を定義する。
- **Phase 4d（C8 ピッチ自己発振 / WASM SIMD / Pick fractional / Look-ahead limiter / LFO 波形拡張 / LFO destinations 拡張 / 楽器切替 fade-out / Cross-tab preset / Preset JSON I/O 等）は別計画扱い**: 本書には Phase 4d の決定事項を含めない。Phase 4c 完了後の retrospective を経て、別仕様書ディレクトリ `docs/specs/<YYYY-MM-DD>-007-phase4d/` で策定する。

## Phase 4c の完成像

> **ブラウザで動作する Rust/WASM 製の物理モデリング多楽器シンセ。Phase 4b で実装した Piano が「弦楽器寄り」だった構造的限界を、Weinreich 1977 / Bank 2000 / Boutillon 1988 / Stulov 1995 / Young 1952 / Conklin 1996 系列の文献に基づき DSP 構造を拡張して解消する。1 Piano voice あたり 1/2/3 本の並列 Karplus–Strong 弦（鍵盤位置に応じ A0〜A1 で 1 弦、A#1〜B2 で 2 弦、C3〜C8 で 3 弦）を持たせ、各弦は ±1.5 cents の unison detuning で配置することで beating と two-stage decay を自然に再現する。打鍵時のフェルトハンマー力プロファイルを Boutillon/Stulov 非線形バネの近似として raised cosine impulse + velocity-dependent cutoff (800〜5500 Hz, 接触時間 1.2〜4 ms) で表現、強打鍵時の brightness 変化を明示化する。さらに `ResonanceBus` (グローバル resonance bus + LPF + 低 gain feedback) を導入し、Sustain ペダル ON 時に全 voice 出力が bus 経由で各 voice ループに微弱に戻ることでペダル音の余韻を表現する。Inharmonicity 係数 B は Phase 4b の A4 基準固定値から MIDI 88 鍵分の f32 LUT (`INHARMONICITY_B_CURVE_PIANO`) へ拡張し、低音 ~3×10⁻⁴ / 高音 ~5×10⁻² の実機曲線を反映。Phase 4a / 4b 互換性は `n_strings = 1` 経路で完全保持（Default kind の出力バイト一致を `tests/fixtures/phase4a_default_c4_v08.rs` で継承）。Piano プリセット聴感チューニングを Step 17-19 で反復実施し、「本物のピアノに近づいた」とユーザー実機聴感で確認できることを完了条件に含める。**

「物理ベースのピアノ音色を獲得した多楽器シンセ」（Phase 4b ゴール）から「本格ピアノ音色を獲得し聴感も納得度の高い多楽器シンセ」へ進める。Phase 4b retrospective §7 推奨スコープの「本格ピアノ音色」を主目的、補助として「F38b 実機計測値取得」「Piano プリセット聴感チューニング」を採用、その他 10 候補（C8 自己発振、WASM SIMD、Pick fractional、Look-ahead、LFO 拡張 (波形/destinations)、Fade-out 状態機械、Cross-tab、Preset I/O 等）は Phase 4d 以降に送る。新規楽器（管楽器 / 打楽器）は引き続き Phase 5 以降。

## ゴール

- **Multi-string per voice**: `KarplusStrong` 内に `[StringState; 3]` + `n_strings_active: usize` を追加（heap 確保ゼロ、案 α）。`n_strings(midi: u8) -> u8` 関数で MIDI ノートに応じ 1 / 2 / 3 を返す（鍵盤位置依存、§3.2）。`note_on` で各弦に detune を適用し弦個別の `adjusted_length` を算出。`process_sample` で N 弦の KS ループを並列実行 + 加算で 1 値を返す
- **Unison detuning**: 中央弦 ±0、左右弦 ±1.5 cents 固定（Piano プリセット内パラメータ `unison_detune_cents`）。3 弦の場合は中央 + 左右、2 弦の場合は中央 + 片側のみ（pre-research §3.4）
- **Hertz law raised cosine hammer**: `note_on_internal` の dispersion_active 経路で buffer 初期化を変更:
  - 接触時間 `t_c_ms = 4.0 - 2.8 × velocity` (1.2 ms 〜 4 ms)
  - cutoff `f_c_hz = 800 + 4700 × velocity` (800 〜 5500 Hz、Phase 4b の 4000 Hz 上限を拡張)
  - amplitude = `velocity.sqrt()` (perceptual loudness 補正)
  - raised cosine 半周期 (`sin²(πi/t_c_samples)`) で接触時間を表現してから velocity LPF を適用
- **Global sympathetic resonance bus**: `dsp-core/src/resonance_bus.rs` を新規実装、delay line (2 ms = 96 sample @ 48 kHz) + LPF + feedback gain (SmoothedValue)。**既存 `Engine::process` ブロック関数の per-sample loop 内**で `VoicePool::process_sample_with_feedback(bus_out_prev, feedback_gain)` 経由で voice 出力を sum → `resonance_bus.process(sum)` で bus_out を更新 → 次 sample で各 voice に 0.03〜0.05 の gain で inject。Piano kind 以外では `feedback_gain = 0`（Phase 4a 互換維持）
- **88 鍵 Inharmonicity B(note) LUT**: `params.json` の Piano エントリに `inharmonicity_b_curve: [f32; 88]` フィールドを追加（A0=21 から C8=108 まで、Young 1952 / Conklin 1996 fitting）。`gen-params.mjs` で `pub const INHARMONICITY_B_CURVE_PIANO: [f32; 88]` を出力。`dispersion::b_curve_piano(midi)` が `midi.clamp(21, 108) - 21` を index に LUT を引く（MIDI 範囲外でも端値 fallback で安全）。Engine の `inharmonicity_b_for_note: fn(u8) -> f32` 関数ポインタ経由で楽器ごとに切替（Piano = `b_curve_piano`、他 = `b_curve_zero`）
- **公開 API は Phase 4b と完全同型**: `Voice::note_on(freq_hz, velocity)` / `KarplusStrong::note_on_with_id(midi, freq_hz, velocity)` のシグネチャは Phase 4b 同等で維持。Phase 4c の楽器パラメータ（`unison_detune_cents` / `inharmonicity_b` / `hammer_cutoff_*`）は `KarplusStrong::set_instrument_params(...)` を **`note_on_with_id` の直前に呼ぶ** ことで内部フィールドに保持し、`note_on_internal` が読み出す。Engine から `VoicePool::note_on_with_piano_params(...)` を呼ぶことでこの 2 段呼出が voice 内に閉じる
- **VoicePool::voices は private 維持**: Phase 4c の Sympathetic 注入は `VoicePool::process_sample_with_feedback(bus_out_prev, feedback_gain) -> f32` を新規追加して voice 配列に直接触らない。内部で `inject = bus_out_prev × feedback_gain` を各 voice に注入後、Phase 2 D20 の `poly_scale = 1/√N` を最後に掛けて返す
- **Piano プリセット聴感チューニング**: Step 17-19 で `damping` / `brightness` / `bodyWet` / `sympathetic_amount` / `unison_detune_cents` / `hammer_cutoff_*` / Modal Body 係数の `gain` / `Q` を反復調整（retrospective §6 教訓「実機聴感確認を必須項目化」の組込み）
- **F38b 実機計測値取得**: Phase 4b 持ち越し、Step 1 で `__synthDev.measureProcessTime(5000)` を Console から実行し Phase 4b ベースライン（Piano 0.047 ms / 非 Piano 0.029 ms）を実機確認、Step 20 で Phase 4c 完了状態の Piano timing を再計測
- **新規 ParamId / C ABI 関数追加なし**: `unison_detune_cents` / `sympathetic_amount` / `inharmonicity_b_curve` は Piano プリセット内のフィールドで完結、UI 露出は Phase 4d 送り。required exports 19（Phase 4a / 4b 同一）を維持
- **Phase 4a / 4b 互換性の機械保証**: `n_strings = 1` のとき Phase 4b と byte 一致（Default kind は dispersion_active=false で Phase 4a HEAD と byte 一致継承、`tests/fixtures/phase4a_default_c4_v08.rs` を Phase 4c でも使用）

## 含めない（明示的に Phase 4d 以降送り）

| 項目 | 理由 |
|---|---|
| C8 ピッチ自己発振 (damping=1.0 経路 / FFT estimator) | ピアノとは別軸の damping 物理限界、Phase 4d で「中規模補助イテレーション」として単独着手（pre-research §9） |
| WASM SIMD (`target-feature=+simd128`, `f32x4`) | CPU 余裕 36×、Phase 4a byte 一致テスト崩壊リスク、Phase 4d 送り（pre-research §10）。PoC は独立ブランチ `phase4c-simd-poc` で先行計測可 |
| Pick position fractional 化 | Piano は hammer 固定位置で pick 概念なし、他楽器のリアリティ向上は Phase 4d |
| Look-ahead limiter (5 ms 遅延、soft clip より透明) | Multi-string で voice 数が増えるため peak 制御強化は Phase 4d 候補 |
| LFO 波形拡張 (S&H / Square / Sawtooth) | Phase 4a §3.3 の保留継続、Phase 4d |
| LFO destinations 拡張 (Pick / Damping / BodyWet) | 同上、Phase 4d |
| 楽器切替の fade-out / cross-fade (`PendingInstrumentChange` 状態機械) | Phase 4b D63 改訂で Phase 4c 送りとした項目、本格ピアノとは独立、Phase 4d 単独テーマ |
| Cross-tab preset 同期 (`storage` event) | UX 需要が出れば Phase 4d |
| Preset JSON file import / export | localStorage v1 で当面十分、Phase 4d |
| Mono + Sustain 本実装 | Phase 2 D29 / Phase 3 D40 / Phase 4a D55 / Phase 4b 継承、Phase 5 |
| Bridge coupling (Multi-string 案 B) | Step 14 の聴感判断で Phase 4c 内 or Phase 4d 送りを決定 |
| Modal Body M=16 拡張 | Step 14 の聴感判断で Phase 4c 内 or Phase 4d 送りを決定 |
| 複数 Piano 機種プリセット (Grand / Upright / Honkytonk) | 本格ピアノ 1 種で実機検証、複数化は Phase 4d 以降 |
| Hammer Hardness UI 露出 | Piano プリセット 1 種で固定、UI 露出は Phase 4d |
| Una corda (ソフトペダル) | Multi-string 1 弦 mute で実装可、Phase 4d 候補 |
| Sympathetic を Sitar / Guitar へ適用 | Phase 4c は Piano kind のみ、楽器横断は Phase 4d |
| Longitudinal string mode (phantom partial) | Bank-Sujbert 2005、Phase 4d 以降 |
| 管楽器 / 打楽器 | Phase 5 領域 |
| 録音・MIDI export | Phase 5 領域 |
| Voice State SAB 化 | COOP/COEP 必要で GitHub Pages 不可（Phase 4a 継承） |

## 受け入れ基準（完了の定義、cargo / clippy / 聴感）

Phase 4c は以下を満たすときに完了とみなす:

1. **cargo / clippy 全 green**:
   - `cargo test -p dsp-core` で **Phase 4b 148 PASS + 1 IGNORED に加え Phase 4c 新規 ~30 件が全て pass**
   - `cargo clippy --workspace --all-targets -- -D warnings` で warning ゼロ
   - `cargo build --release --target wasm32-unknown-unknown` で WASM ビルド成功
2. **WASM 性能予算内**:
   - gzip ≤ 22 KB（target 20 KB、警戒 25 KB、撤退 30 KB）
   - Worklet bundle ≤ 12 KB
   - `process` per call (Piano, release cargo timing, 128 frames @ 48 kHz) < 0.15 ms
   - `process` per call (非 Piano) < 0.05 ms（Phase 4b と同等、regression なし）
3. **Phase 4a / 4b 互換性**:
   - Default kind + Mod Wheel=0 で Phase 4a HEAD (commit dfa81c3) と ε=1e-6 バイト一致継承
   - Piano 以外の 7 楽器 (Default + Guitar Classical / Ukulele / Mandolin / Bass / Guitar Steel / Sitar / Piano-phase4b は意図的不一致) で Phase 4b 同等の出力
4. **ヒープ確保ゼロ**: `process` ホットパスで alloc ゼロ（`test_no_allocation_in_process` 通過）
5. **C ABI 互換**: 19 required exports すべて維持、`synth_apply_instrument(handle, 7)` で Piano 切替動作
6. **実機聴感確認**:
   - Step 19 でユーザーが `pnpm dev` で Piano プリセットを試聴し、「Phase 4b より本物のピアノに近づいた」と判断
   - 既存 7 楽器に regression なし（pre-research §3.7 / §11 で確認）
7. **F38b 実機計測**:
   - Phase 4b ベースライン (Piano 0.047 ms / 非 Piano 0.029 ms) を Step 1 で再確認
   - Phase 4c 完了後 (Piano < 0.15 ms / 非 Piano < 0.05 ms) を Step 20 で確認

## Phase 4c の設計判断（D68〜D85、18 項目）

Phase 4c で行う設計判断を D タグで管理する。Phase 4b の D67 までを継承、Phase 4c は D68 から開始。

| ID | 判断 | 採用根拠 |
|---|---|---|
| **D68** | **Phase 4c 主目的 = 本格ピアノ音色 (Multi-string + Hertz hammer + Sympathetic + B(note))** | retrospective §7 の聴感負債が最大、文献的に確立した手法、構造拡張で他楽器にも波及可能（pre-research §2） |
| **D69** | **Multi-string の弦数戦略 = `n_strings(midi)` 関数で鍵盤位置依存 (1/2/3)** | Steinway D 等の標準ピアノ設計に倣う、A0〜A1 で 1 弦、A#1〜B2 で 2 弦、C3〜C8 で 3 弦（pre-research §3.2） |
| **D70** | **Voice 設計案 α (`KarplusStrong` 内に `[StringState; 3]` + `n_strings_active: usize`)** | Voice trait dyn 化を避ける、VoicePool 固定型を維持、`n_strings = 1` で Phase 4b 経路と byte 一致継承可能（pre-research §3.5） |
| **D71** | **Multi-string buffer = 案 1 (弦ごとに独立 Vec<f32>)** | 各弦に独立 dispersion / damping / loss の余地、Step 9 で memory growth が問題出れば案 2 (共有 buffer + 3 read 位置) へ移行（pre-research §3.6） |
| **D72** | **Unison detuning = 中央弦 ±0、左右弦 ±1.5 cents 固定** | Weinreich 1977 / Galembo-Askenfelt の実測典型値、Piano プリセット内パラメータ `unison_detune_cents` で保持（pre-research §3.4） |
| **D73** | **Bridge coupling は Step 14 の聴感判断で採否決定（案 A 単純加算から開始、必要なら案 B 追加）** | 案 A だけでも beating + two-stage decay の代理が得られる、案 B は CPU +10〜20%、Phase 4c 内追加 or Phase 4d 送りを Step 14 で判断（pre-research §3.3） |
| **D74** | **Hertz hammer 方式 C: 接触時間 + raised cosine impulse + velocity LPF** | per-sample ODE (案 B) は同時打鍵時の瞬間 CPU spike リスク、Phase 4b の Commuted impulse 枠組みを拡張する案 C が費用対効果良（pre-research §4.2） |
| **D75** | **Hertz hammer のパラメータ式: t_c_ms = 4.0 - 2.8·v, f_c_hz = 800 + 4700·v, amp = sqrt(v)** | Boutillon 1988 / Stulov 1995 数値解の近似、Phase 4b の固定 cutoff 4000 Hz → 5500 Hz に上方拡張（pre-research §4.3） |
| **D76** | **Sympathetic resonance 方式 A: Global resonance bus + feedback** | Bank 2000 系の標準、O(N) で済む、Step 14 で必要なら band-split bus (案 C) へ拡張（pre-research §5.2） |
| **D77** | **Sympathetic は Piano kind のみ active、他楽器は `feedback_gain = 0`** | Phase 4a / 4b 互換維持、Sitar / Guitar への展開は Phase 4d（pre-research §5.9） |
| **D78** | **B(note) LUT 方式 B: 88 鍵 × f32 LUT (`INHARMONICITY_B_CURVE_PIANO`)** | Young 1952 / Conklin 1996 実測カーブを直接埋める、関数 A の k 値選定の試行錯誤を回避、+0.1 KB gzip（pre-research §6.3） |
| **D79** | **B(note) LUT 値は Step 4 で概数を確定、Step 18-19 の聴感調整で精密化** | 仕様書策定時には Young 1952 図 1 からの fitting を完了させず、概数 + 聴感調整で詰める（pre-research §6.7） |
| **D80** | **Modal Body M=16 拡張は Step 14 の聴感判断で採否決定（M=8 から開始）** | Multi-string + Sympathetic + B(note) で聴感改善が支配的、M=16 の必要性は実装後判断（pre-research §7.2） |
| **D81** | **新規 ParamId / C ABI 関数追加なし** | `unison_detune_cents` / `sympathetic_amount` / `inharmonicity_b_curve` はプリセット内、UI 露出は Phase 4d。required exports 19 維持 |
| **D82** | **Piano プリセット聴感チューニング (Step 17-19) を完了条件に含める** | retrospective §6 教訓「cargo / clippy 全 green でも聴感確認で違和感が出る (Phase 4b の Piano)」の組込み、聴感達成までは Phase 4c 完了とみなさない |
| **D83** | **Phase 4a HEAD byte 一致継承 (`n_strings = 1` で機械保証)** | Phase 4b D67 の `dispersion_active=false` byte 一致を引き継ぎ、Phase 4c でも `tests/fixtures/phase4a_default_c4_v08.rs` を維持（pre-research §3.7） |
| **D84** | **C8 自己発振 / WASM SIMD / Pick fractional / Look-ahead / LFO 拡張 / Fade-out / Cross-tab / Preset I/O は Phase 4d 送り** | Phase 4c の実装規模 (18→22 Step) を肥大化させない、Phase 4d で「中規模補助イテレーション」として纏める（pre-research §8 / §9 / §10） |
| **D85** | **F38b 実機計測値取得を Step 1 (ベースライン) + Step 20 (Phase 4c 後) で実施** | Phase 4b 持ち越し（retrospective §持ち越し）、ユーザー操作必須のため Auto mode 完結不可、`pnpm dev` + Console API で取得 |

## 仕様書外で発生し得る判断

実装段階で以下のような事象が発生した場合、仕様書改訂を実装と同 commit で行う運用とする（Phase 3 D36 / Phase 4a Triangle 式 typo / Phase 4b `test_dispersion_b_zero_limit` 改訂で 3 回連続発生、定石化）:

- B(note) LUT 値が実機聴感で違和感ある（Step 19 で発覚）→ LUT 値を改訂 + 仕様書 §6.4 / §11.4 を更新
- Multi-string buffer (案 1) で memory growth が CI で問題 → 案 2 (共有 buffer) へ移行 + 仕様書 §3.6 / §11.4 を更新
- Step 14 の聴感判断で Modal M=16 / bridge coupling が必要 → 該当 Step を追加 + 仕様書 §7.2 / §3.3 / §11 を更新

---

## まとめ

Phase 4c は Phase 4b の最大負債である「Piano 音色が弦楽器寄り」を **Multi-string + Hertz hammer + Sympathetic resonance + B(note) LUT** の構造拡張で正面から解消する。CPU 予算 +0.035 ms (Multi-string + Sympathetic、Modal M=8 維持) と WASM gzip +1.2 KB は Phase 4b で確保した余裕 (target 1.7 ms の 4.8%、gzip 25 KB から 5 KB 余裕) 内に収まる。Phase 4a / 4b の byte 一致互換は `n_strings = 1` 経路で機械保証継承、新規 ParamId / C ABI 追加なし。実装着手前に §12 (pre-research) の 15 件をユーザー承認、聴感達成 (Step 17-19) を完了条件に含めることで「仕様書ドリブン + 聴感確認」の二軸開発を確立する。Phase 4d は C8 自己発振 + 補助多項目 (Pick fractional / Look-ahead / LFO 拡張 / Fade-out / Cross-tab / Preset I/O) 等で別計画。
