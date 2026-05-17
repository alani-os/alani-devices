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
cargo test --no-default-features
cargo check --no-default-features
cargo clippy --all-targets --all-features -- -D warnings
python3 tools/validate_devices_examples.py
```

## Repository Layout

The crate is `no_std` by default and dependency-free. It currently exposes fixed-capacity Rust contracts for:

- `classes`: device classes, capability bits, operation opcodes, built-in class descriptors, and mock profiles.
- `registry`: descriptor registration, duplicate prevention, capability-gated open requests, device call envelopes, and deterministic mock devices.
- `interrupt`: interrupt binding, top-half acknowledgement records, deferred events, and bounded event queues.
- `dma`: pinned DMA buffer descriptors, mapping states, and policy validation for disabled, bounce-buffer, pinned, and IOMMU-required modes.
- `schemas/devices-catalog.schema.json`: stable metadata for device classes, operations, capabilities, rights, DMA policies, interrupt policies, and redaction states.
- `examples/devices-catalog.json`: checked catalog example covering all built-in device classes.
- `tools/validate_devices_examples.py`: dependency-free validator for checked-in catalog examples.

## Public Contracts

- `DeviceCall` carries operation, buffer bounds, trace, budget, data-class, and redaction metadata.
- Cognitive operations require explicit `DeviceCallBudget` and present `TraceContext` before validation succeeds.
- `DeviceCallResult` includes deterministic `DeviceUsage` and `DeviceProvenance` metadata when a mock or driver returns data.
- Security-sensitive operations fail closed for reserved bits, unauthorized opens, unsupported calls, invalid buffers, unpinned DMA, DMA policy violations, invalid interrupts, unredacted sensitive payloads, and device faults.

## Feature Flags

- `std` is enabled by default for host-mode tests.
- `--no-default-features` builds the library as `no_std`.

Keep public API changes synchronized with `docs/repositories/alani-devices.md`, Doc 42, Doc 43, and Docs 13-14.
