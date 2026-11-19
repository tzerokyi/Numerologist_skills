# HANDOFF — Native Rust core migration

**Date:** 2026-11-19 (Asia/Seoul)
**Repository:** https://github.com/tzerokyi/Numerologist_skills
**Upstream:** https://github.com/FANzR-arch/Numerologist_skills
**Migration branch:** `feat/native-rust-core`
**PR:** https://github.com/tzerokyi/Numerologist_skills/pull/1
**Requested disposition:** document the incomplete work and merge the implementation into `main` *without representing it as validated*. The owner explicitly deferred build/runtime verification.

## Current delivered code

- Native, headless Rust library `numerologist_core` in `src/lib.rs`.
- JSON CLI `numerologist` in `src/main.rs`: `qimen`, `ziwei`, `bazi`, `doctor`; file or stdin (`--input -`), `--json` and `--pretty`.
- `src/calendar.rs`: shared structured civil time parsing, calendar conversion, Ganzhi/Xun helpers.
- `src/qimen.rs`: time Qimen (24 JieQi, Yin/Yang, Ju/Yuan, Earth/Sky, stars, doors, gods, center-hosting, Zhifu/Zhishi, Yima, Xunkong, traditional pattern labels).
- `src/ziwei.rs`: Ziwei natal palaces, stars, Sihua, five-element bureau, decadal and yearly layers; late-Zi and leap month options.
- `src/bazi.rs`: four pillars, Ten Gods, hidden stems, optional luck direction, selectable 23:00 day policy.
- `fixtures/*.json`: representative CLI inputs; `tests/iztro_parity.rs`: independent iztro 2.6.2 JSON corpus test.
- `.github/workflows/native-core.yml`: intended trusted self-hosted Linux runner, format/lock, strict Clippy, tests and CLI smoke.
- `docs/RUST_MIGRATION.md` and `AGENTS.md`: architecture, rules, API and development contract.

**Source preservation:** Upstream `qimen-dunjia/`, `ziwei-doushu/`, `bazi/` including Python tests, source Python CLIs, old HTML and original references remain. This means the *repository as a whole* is **not** Rust-only; the *new CLI runtime* is intended to be fully native Rust, using a native Rust calendar dependency. No GUI/frontend needed.

## Verification and release status — IMPORTANT

- **Not verified**: no successful Rust compile, format, Clippy, tests, linked CLI or parity gate has been observed for the current revision.
- On 2026-11-19, self-hosted GitHub Actions was queued/pending; earlier runs were cancelled. A queued/cancelled run **is not** a pass. Check Actions on `main` for a run at the exact SHA; do not infer success from earlier branches.
- **No Cargo.lock in the migration commits yet.** Runner workflow intends to generate and commit the lockfile on `feat/native-rust-core`. This mechanism must be reworked once `main` becomes the active branch: do not auto-commit or mutate unreviewed changes on `main`.
- `Cargo.toml`: `publish = false` deliberately. The original upstream has **no explicit LICENSE**; get permission / determine licensing before publishing any crate. `lunar-rs` is a pinned Git-revision dependency; replace with a verified published dependency prior to crates.io.
- Toolchain `1.99.1` and labels `[self-hosted, linux]` come from the owner's delivery policy; verify this new repository's access to the runner. Do not switch to untrusted fork PR execution, delete caches or commit binaries.
- Windows compilation/runtime not tested; no GUI testing applies.

## Outstanding work (in priority order)

1. **Unblock runner**: confirm the repository can use the owner's authorized self-hosted runner; check labels and branch/workflow permissions. Do not run third-party fork PR code on owner hardware.
2. **Fix the main-branch verification lane**: generate lockfile on a new `feat/` branch (never auto-mutate main) or submit a deliberate PR containing `Cargo.lock`; use the runner to apply exact rustfmt and verify the final committed SHA.
3. Run `cargo fmt --all -- --check`, `cargo clippy --locked --all-targets -- -D warnings`, `cargo test --locked --all-targets`, `cargo build --locked --bin numerologist`, then execute actual CLI JSON smoke inputs (`fixtures/`).
4. **Source parity**: compare original Python Qimen golden tests and the full Ziwei iztro JSON corpus. Check 24-term incl. Zhongqi boundaries, Yin-god reversal, Zhifu/Zhishi separation, center palace, XunKong, lunar leap days (split 15/16), 23:00 crossover, 14-star pairing, Sihua/brightness, decadal order, and timezone assumptions.
5. The Rust BaZi module **does not** yet implement the original interpretive rule set for day-master strength, useful/unfavorable elements, chart structure, start age of 10-year luck cycle, or full flowing years. Port only deterministic, rule-grounded parts; do not invent predictive certainty.
6. Clarify timezone policy: the new API currently accepts *civil local time* and carries the timezone string as metadata. The upstream `calendar_type=now` and location-based timezone selection are intentionally not implemented. Either preserve semantics through a deliberate clock/TZ adapter or document incompatibility with tests.
7. Stabilize public API/schema/semver, enrich structured errors, improve strict input validation and Windows build evidence before labeling as reusable or publishable.
8. Only **after verified parity**, distill reusable Python/calendar migration findings into `tzerokyi/SKILLS`'s `skills/rustify/SKILL.md` (or relevant references). Do **not** edit `rustryx`, which is a reference-only repository. No reusable skill changes were applied during this unverified port.

## API + CLI handoff

```sh
cargo run --bin numerologist -- --help
cargo run --bin numerologist -- doctor --json
cargo run --bin numerologist -- qimen --input fixtures/qimen-yang.json --json
cargo run --bin numerologist -- qimen --input fixtures/qimen-yin.json --json
cargo run --bin numerologist -- ziwei --input fixtures/ziwei.json --json
cargo run --bin numerologist -- bazi --input fixtures/bazi.json --json
cat fixtures/ziwei.json | cargo run --bin numerologist -- ziwei --input - --pretty
```

Native library entrypoints: `qimen::calculate(&QimenRequest)`,
`ziwei::calculate(&ZiweiRequest)`, and `bazi::calculate(&BaziRequest)`.
All return typed `Result<T, NumerologistError>`; JSON CLI uses the same core.
No HTTP service, mandatory MCP, interactive prompt, Python/JS runtime or frontend.

**Do not interpret this handoff or a merged main branch as proof of 100% behavioral parity.**
