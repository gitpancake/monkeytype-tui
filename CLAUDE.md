# CLAUDE.md

## Project

Terminal client for monkeytype.com. Runs in a tmux pane. WPM/acc/consistency computed locally; results optionally POSTed to `api.monkeytype.com/results` with ApeKey auth.

Rust binary, installed via `cargo install --path .` → `~/.cargo/bin/monkeytype`.

## Layout

- `src/main.rs` — clap CLI, dispatches to UI, then submits if `MONKEYTYPE_APE_KEY` set.
- `src/ui.rs` — ratatui render + crossterm event loop. Pure UI, no stats math.
- `src/test.rs` — `Test` state machine + `TestResult`. WPM/acc/consistency math.
- `src/words.rs` — embedded english-200 list (mirrors monkeytype's english-200 set).
- `src/api.rs` — `submit_result` → POST `/results` with `Authorization: ApeKey ...`.
- `src/config.rs` — env-var loader (only `MONKEYTYPE_APE_KEY` for now).

## Stats formulas

Match monkeytype where possible (see `frontend/src/ts/test/test-stats.ts` upstream).

- `wpm = (correct_chars / 5) / (duration / 60)`
- `raw_wpm = (total_typed / 5) / (duration / 60)`
- `accuracy = correct / total * 100`
- `consistency = (1 - stddev/mean) * 100` per-second WPM samples — simpler than monkeytype's Kovalchik-scaled formula. Port `kogasa` from `frontend/src/ts/utils/misc.ts` if you need parity.

Spaces between completed words count as one correct char each (monkeytype convention).

## Result submission gotchas

`POST /results` is undocumented and validated strictly. Common rejection causes:

- `chartData.wpm/raw/err` arrays must be non-empty and match `Math.round(testDuration)` length.
- `keySpacingStats` / `keyDurationStats` need real averages — anti-cheat flags zeros. To produce real values, capture per-keystroke `Instant` in `Test::type_char` and compute mean/sd in `finalize`.
- `mode2` is a string even for time/words ("30", not 30).
- `language` must match a known monkeytype language ("english", "english_1k", etc.).

If you get 4xx, the response body's `message` field tells you which field failed validation. Log it.

## Editing rules

- After editing `src/*.rs`, run `cargo fmt && cargo clippy --all-targets -- -D warnings` before commit — CI enforces both.
- After editing `Cargo.toml`, also commit `Cargo.lock`. Binary convention is lock-committed.
- `cargo install --path .` reinstalls the `monkeytype` binary into `~/.cargo/bin/`. Run after every change you want available shell-wide.
- Don't add deps casually — build time matters for an interactive tool. Current cold build ~30s; keep it tight.

## Things deliberately NOT done

- No quote mode, funbox, punctuation/numbers toggles. Add only if used.
- No PB tracking client-side — server returns `isPb` on submission.
- No language switching — english-200 only. Pull other languages from upstream's `frontend/static/languages/*.json` if needed.
- No theming. Pending-char color is `Rgb(110, 110, 110)`; tweak in `src/ui.rs` `draw()` if visibility regresses.
- No per-key timing capture (see "Result submission gotchas").

## Test the binary

```sh
cargo run --release -- --time 30 --no-sync
cargo run --release -- --words 25 --no-sync
```

For tmux usage: install with `cargo install --path .`, then in any pane:

```sh
monkeytype --time 30
```
