# monkeytype-tui

Terminal client for [monkeytype.com](https://monkeytype.com). Runs in any shell or tmux pane. Optionally syncs results to your monkeytype account via ApeKey.

```
┌monkeytype-tui──────────────────────────────────────────────┐
│ time 23.4s / 30s   chars 84                                │
└────────────────────────────────────────────────────────────┘
┌type────────────────────────────────────────────────────────┐
│ the quick brown fox jumps over the lazy dog and runs back  │
│ to the den where it sleeps until morning                   │
└────────────────────────────────────────────────────────────┘
┌────────────────────────────────────────────────────────────┐
│ wpm 78.3   acc 96.4%   esc to quit                         │
└────────────────────────────────────────────────────────────┘
```

## Features

- Time or word-count modes
- Live WPM + accuracy
- Per-second WPM samples for consistency calc
- Optional sync to monkeytype.com via [ApeKey](https://monkeytype.com/account)
- Single binary, no runtime deps

## Install

```sh
git clone https://github.com/gitpancake/monkeytype-tui ~/projects/monkeytype-tui
cd ~/projects/monkeytype-tui
cargo install --path .
```

Installs `monkeytype` into `~/.cargo/bin/`. Make sure that's on your PATH (rustup adds it by default).

## Usage

```sh
monkeytype                       # 30s test, sync if MONKEYTYPE_APE_KEY set
monkeytype --time 60             # 60s test
monkeytype --words 50            # 50-word test
monkeytype --time 30 --no-sync   # local only
```

### Keys

| Key             | Action            |
| --------------- | ----------------- |
| any char        | type              |
| space           | advance word      |
| backspace       | edit current word |
| esc / ctrl-c    | quit              |

## Sync to monkeytype.com

1. Sign in at [monkeytype.com](https://monkeytype.com) → Account → ApeKeys.
2. Generate a key with **result submission** scope.
3. Export it:

   ```sh
   export MONKEYTYPE_APE_KEY=your_key_here
   ```

4. Run a test. On finish the result POSTs to `https://api.monkeytype.com/results`.

If sync fails the error from the backend is printed but local stats still show. The `/results` schema is strict and undocumented — see [CLAUDE.md](./CLAUDE.md) for known gotchas.

## Layout

```
src/
├── main.rs    CLI + dispatch
├── ui.rs      ratatui render + key loop
├── test.rs    state machine, WPM/acc/consistency math
├── words.rs   embedded english-200 list
├── api.rs     POST /results with ApeKey auth
└── config.rs  env-var loader
```

## Known gaps vs real monkeytype

- english-200 only — pull other languages from upstream's `frontend/static/languages/*.json` if needed
- No per-keystroke `keySpacing` / `keyDuration` capture — anti-cheat may reject submissions
- Consistency uses simple `(1 - cv) * 100` instead of monkeytype's Kovalchik-scaled formula
- No quote mode, funbox, punctuation, numbers, themes, PB detection

## Legal

ApeKeys are monkeytype's official mechanism for third-party result submission. Don't use this to submit fake results — the backend has anti-cheat and accounts can be flagged.

## License

[MIT](./LICENSE)
