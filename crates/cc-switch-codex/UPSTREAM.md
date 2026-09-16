# CC Switch compatibility modules

- Source: https://github.com/farion1231/cc-switch
- Pinned commit: `06082e189d65e6d6dbadc35dacdac1ce6c79d89a`
- License: MIT, Copyright (c) 2025 Jason Young; exact license in `LICENSE`.
- This is a source extraction from the upstream Tauri application, not an official upstream SDK. It has no access to Codex configuration, credentials, processes, SQLite or provider HTTP connections.

## What is reused

`src/proxy/providers/` retains upstream request/response conversion, streaming, tool history, tool/schema helpers and their tests. `src/proxy/` retains SSE, JSON/media helpers and circuit breaker. The model-capability registry and three bundled model templates are unchanged. `upstream-files.json` pins each file's SHA-256.

`provider.rs` contains the original reasoning configuration type; `transform.rs` contains the three helpers needed by the Chat converter. `catalog.rs` contains the original tool profile, model specification, catalog builders, reasoning-level overrides, official-vendor builder and required-field backfill. The public `entry()` function in `catalog_adapter.rs` is a local adapter. Explicit parallel-tool overrides also apply to Chat in this adapter. The original extracted blocks are separately checksummed.

`lib.rs` exposes a small API and supplies the minimum error/config/type shapes required by those modules. Upstream's application database, provider router and generic HTTP forwarder are not imported wholesale. The core crate supplies the HTTP/credential lifecycle and explicit ordered queue; the circuit breaker itself is upstream code.

Provider metadata is selected from `src/config/codexProviderPresets.ts`. `packages/provider-registry/upstream.json` records the source and selected rows. Local IDs, existing key/documentation links and legacy model candidates are retained. No upstream provider-test claim is inherited.

## Deliberate local differences

- Credentials remain in the system vault; changes to provider/address/full-URL scope require an explicitly supplied key. Only the selected route's key is sent to its endpoint. Redirects are disabled.
- Tool history is restricted to the client alias, 64 responses, one hour and 1 MiB per entry. Conflict checks and whole-history expansion wrap the upstream call/metadata restoration. It is never persisted.
- Chat input guards reject orphaned/duplicate/out-of-order tool results. Stream framing is bounded and malformed JSON, premature EOF, undeclared tools and malformed final tool arguments fail closed. Token-truncated tool calls are not published as completed executable calls. These guards wrap upstream; the vendored converter is unchanged.
- Native Responses bodies/events are passed through. For `/responses/compact`, native routes forward to that endpoint and Chat routes use `/chat/completions`, as upstream does. No fake encrypted compaction item is produced.
- The local wrapper now rejects remote V2 `compaction_trigger` and opaque `compaction` input items on Chat routes instead of silently dropping them. Native Responses preserves these items and their JSON/SSE output. Ordinary text-summary requests, including compaction metadata, still use the normal bridge. This guard does not import the unmerged upstream PR #5536 or implement its envelope protocol.
- Remote compaction is an explicit global preference because this app writes one shared Codex provider table. It uses upstream's `name = "OpenAI"` marker; disabled/default is `name = "Codex Switch"`. The config transaction and exact rollback remain local.
- HTTPS external upstreams only; base mode retains prefixes and normalizes known endpoint suffixes. Full URL mode retains path/query; compact derives only from a known `/responses` ending. Opaque full endpoints fail with an actionable error. Local HTTP is used only by synthetic tests.
- The explicit fallback queue uses each configuration's first model, endpoint, protocol and vault credential, in the displayed order (maximum eight, one attempt each). Without a queue, no downgrade occurs. Retryable pre-response statuses: 401, 403, 404, 405, 408, 429, 500, 502, 503, 504; network/JSON/initial-stream errors become 502. Local invalid requests do not retry. A committed stream is never replayed. This bounded HTTP policy is a local adapter, not a claim that the entire upstream forwarder was imported.
- Remote compaction and fallback default off. Saved configurations do not get silently migrated to newer preset protocols/models/addresses.

## Updating

1. Review a specific upstream commit, license and changes to these modules together, including their dependent helpers and tests.
2. Copy entire `kind=verbatim` files without formatting/reworking them. Re-extract marked fragments and review the local API/type adapters separately.
3. Refresh the manifest and registry source hashes only after comparing the actual source. Update this document for local behavioral differences.
4. Run `pnpm check:upstream`, `pnpm test`, core Clippy, desktop checks/build and isolated integration/browser checks. Do not infer provider/App compatibility from unit tests.
5. Provider probes and actual Codex App long-context/tool/compaction validation remain separate evidence. Never turn preset metadata into a statement that an account or endpoint was tested.
