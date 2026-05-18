# monkeytype-tui

Terminal typing tester. Runs in a tmux pane. Optionally syncs results to your monkeytype.com account via ApeKey.

## Status

Scaffold. Local test loop works (TUI + WPM/acc/consistency). Result sync is best-effort — monkeytype's `/results` schema is strict and undocumented; expect to iterate on the payload in `src/api.rs` against live 4xx responses.

## Install rust

```sh
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source "$HOME/.cargo/env"
```

## Build + run

```sh
cd ~/projects/monkeytype-tui
cargo run --release -- --time 30
cargo run --release -- --words 50
cargo run --release -- --time 60 --no-sync
```

## Sync to monkeytype

1. Log into monkeytype.com → Account → ApeKeys → generate one with **result submission** scope.
2. `export MONKEYTYPE_APE_KEY=...` (or put it in `.zshenv`).
3. Run a test. On finish the result POSTs to `https://api.monkeytype.com/results`.

If sync fails, the error from the backend is printed but local stats still show.

## Keys

- type to start
- backspace edits current word
- space advances word
- esc / ctrl-c quits

## Files

- `src/main.rs` — CLI + dispatch
- `src/ui.rs` — ratatui render + key loop
- `src/test.rs` — typing state machine, WPM/acc/consistency math
- `src/words.rs` — embedded english-200 list
- `src/api.rs` — monkeytype `/results` client (ApeKey auth)
- `src/config.rs` — env-var config

## Known gaps vs real monkeytype

- Word list is english-200 only. To match monkeytype: pull `frontend/static/languages/english.json` from the upstream repo at build time.
- No `keySpacing` / `keyDuration` arrays — anti-cheat may reject submissions. Wire per-keystroke timing in `Test::type_char`.
- Consistency uses simple `(1 - cv) * 100`. Monkeytype uses a Kovalchik-scaled formula — port from `frontend/src/ts/utils/misc.ts:kogasa`.
- No quote mode, no funbox, no punctuation/numbers toggles.
- No PB detection (server returns `isPb`; we just forward).

## Legal

ApeKeys are monkeytype's official mechanism for third-party result submission. Don't use this to submit fake results — the backend has anti-cheat and your account can be flagged.
