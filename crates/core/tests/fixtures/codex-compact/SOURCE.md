# Codex summary test fixtures

- Source: [openai/codex, rust-v0.149.0](https://github.com/openai/codex/tree/rust-v0.149.0/codex-rs/prompts/templates/compact).
- Files: `prompt.md` and `summary_prefix.md`, copied without edits.
- License: Apache-2.0; see adjacent `LICENSE`.
- Used only by the opt-in provider test binary. Production compaction remains the Codex client's responsibility.

The test exercises summary generation and consumption through real provider requests. It does not run the installed Codex client or reproduce its entire history-retention algorithm. Only the generated summary and a new question survive into the test continuation; original user/assistant history and baseline answers are not replayed.
