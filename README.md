# alani-devices

Hardware and virtual-device traits, registration types, polling abstractions, IRQ handoff, DMA descriptors, and device classes.

| Field | Value |
|---|---|
| Tier | MVK required |
| Owner | Device team |
| Aliases | None |
| Architectural dependencies | `alani-lib`, `alani-abi`, `alani-platform`, `alani-observability` |

## Quick start

```bash
cargo fmt -- --check
cargo test --all-features
```

## Public skeleton

The crate is `no_std` by default and dependency-free. It currently exposes fixed-capacity Rust contracts for:

- `classes`: device classes, capability bits, operation opcodes, built-in class descriptors, and mock profiles.
- `registry`: descriptor registration, duplicate prevention, capability-gated open requests, device call envelopes, and deterministic mock devices.
- `interrupt`: interrupt binding, top-half acknowledgement records, deferred events, and bounded event queues.
- `dma`: pinned DMA buffer descriptors, mapping states, and policy validation for disabled, bounce-buffer, pinned, and IOMMU-required modes.

Security-sensitive operations fail closed for reserved bits, unauthorized opens, unsupported calls, invalid buffers, unpinned DMA, DMA policy violations, invalid interrupts, and device faults.

Keep public API changes synchronized with `docs/repositories/alani-devices.md`, Doc 42, Doc 43, and Docs 13-14.
