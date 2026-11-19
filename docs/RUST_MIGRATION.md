# Rust-native migration ledger

## Scope and boundaries

This port adds an independent Rust library and CLI while keeping the original
qimen-dunjia/, ziwei-doushu/, bazi/, Python tests and source references untouched.
No frontend, Python runtime, JavaScript, webview, server or MCP transport is used.

| Domain | Native entry point | Scope | Evidence |
| --- | --- | --- | --- |
| Qimen | `qimen::calculate(&QimenRequest)` | 24 JieQi, nine palaces, three yuan, Earth/Sky, doors, stars, gods, XunKong, Yima, structural patterns | Original golden spring/summer and issue #11 encoded as Rust tests; runner pending |
| Ziwei | `ziwei::calculate(&ZiweiRequest)` | 14 major stars, auxiliaries, brightness, Sihua, decadal and annual fortunes, leap and late-Zi rules | Original hand chart / lunar boundary cases encoded; full iztro fixture sweep pending |
| BaZi | `bazi::calculate(&BaziRequest)` | Exact four Ganzhi pillars | Day-master strength, Ten Gods and interpretive fortune rules not yet ported |
| Calendar | `calendar::as_solar` | Solar/lunar conversion and boundary data | `houseme/lunar-rs` pinned Git revision; exact lunar_python parity remains to verify |

The current calendar library is native Rust but a Git dependency. Before
crates.io publishing replace it with a verified published compatible release
or a crates.io-compatible calendar layer. Upstream had no explicit LICENSE;
**do not publish** until the rights holder's license is confirmed.

## CLI and library contract

```sh
cargo run --bin numerologist -- --help
cargo run --bin numerologist -- doctor --json
cargo run --bin numerologist -- qimen --input fixtures/qimen-yang.json --pretty
cargo run --bin numerologist -- qimen --input fixtures/qimen-yin.json --pretty
cargo run --bin numerologist -- ziwei --input fixtures/ziwei.json --pretty
cargo run --bin numerologist -- bazi --input fixtures/bazi.json --pretty
cat fixtures/qimen-yang.json | cargo run --bin numerologist -- qimen --input - --json
```

The CLI returns JSON via stdout and failures via stderr and a nonzero exit code.
No files are changed by a calculation; no GUI is initialized. It accepts
explicit **local civil clock time**; timezone is metadata, *not* an inferred
conversion. The old `calendar_type=now` intentionally returns a clear
error instead of introducing hidden host-clock nondeterminism.

## Verification and remaining gaps

- Self-hosted, trusted-branch-only CI with 12 Cargo build jobs and persistent caches.
- On feat/native-rust-core only, the runner commits exact rustfmt output and
  Cargo.lock to the staged branch, then checks the resulting SHA.
- Checks: `fmt --check`, strict Clippy, tests, linked CLI build and JSON smoke.
- Python's original tests are still an **oracle**, not evidence the Rust tests
  already pass. Compare 24-term transition instants, leap-month split on day
  15/16, 23:00 crossover, center hosted stem, Yin god directions and Sihua.
- Full independent iztro 2.6.2 star-by-star corpus not ported yet.
- Windows compilation/runtime have not been verified. No native OS assumptions
  in the runtime code, but this is not proof of Windows portability.
- No executable or Cargo target/registry cache is checked into Git.

## Rustify feedback to distill after verification

1. Port pinned calendar semantics before domain arithmetic, not merely constants.
2. Source tests are specification oracles; golden border cases prevent drift.
3. Separate the typed native core, stable CLI and optional MCP adapter.
4. Keep historical source and publication/license provenance explicit.
