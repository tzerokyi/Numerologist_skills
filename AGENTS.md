# Numerologist native Rust fork

- The original Python scripts, tests, fixtures and references are the frozen migration oracle. Keep them unchanged.
- Native Rust library and headless CLI only: no frontend, Python runtime, JavaScript, HTML runtime, Tauri, server or FFI wrapper.
- One typed calculation core under src/{calendar,qimen,ziwei,bazi}.rs; CLI and future optional MCP must call it.
- No compiled binaries, target directories or caches in Git. Minimize and audit dependencies.
- Stage Rust source on feat/*, not main; format on the trusted self-hosted runner before promoting.
- Run rustfmt, strict Clippy, regression tests, linked CLI smoke and Windows checks when supported.
- Preserve exact input and calculated behavior, including 24 JieQi, center hosting, Zhifu/ Zhishi separation, leap months, late Zi and 14-star pair rules.
- Upstream license is missing: keep publish=false until rights are resolved.
- Never edit Rustryx; it is a reference archive, not a target.
