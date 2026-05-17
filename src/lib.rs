#![cfg_attr(not(feature = "std"), no_std)]

//! Dependency-free device contracts for the Alani MVK.
//!
//! This crate owns the public skeleton for device classes, registration,
//! capability-gated open/call envelopes, interrupt handoff, DMA descriptors,
//! and deterministic host-mode mock device behavior. Sibling repositories
//! remain represented as Cargo metadata until their public APIs stabilize.

pub mod classes;
pub mod dma;
pub mod interrupt;
pub mod registry;

pub use classes::{
    builtin_class_descriptor, ClassesDescriptor, DeviceCapabilities, DeviceClass,
    DeviceClassDescriptor, DeviceOperation, DeviceOperationSet, MockDeviceProfile,
    BUILTIN_DEVICE_CLASSES, DEVICE_CAP_ACCELERATOR, DEVICE_CAP_AUDIT_STORAGE, DEVICE_CAP_BLOCK_IO,
    DEVICE_CAP_COGNITIVE, DEVICE_CAP_DMA, DEVICE_CAP_ENTROPY, DEVICE_CAP_INTERRUPT,
    DEVICE_CAP_MEMORY, DEVICE_CAP_POLL, DEVICE_CAP_READ, DEVICE_CAP_STREAM, DEVICE_CAP_WRITE,
    DEVICE_OPERATION_COUNT, KNOWN_DEVICE_CAPABILITIES, MAX_DEVICE_NAME_LEN,
};
pub use dma::{
    DmaAddressSpace, DmaBuffer, DmaDescriptor, DmaDirection, DmaMapping, DmaPolicy, DmaState,
    MAX_DMA_ALIGNMENT, MAX_DMA_BUFFER_LEN,
};
pub use interrupt::{
    InterruptAck, InterruptBinding, InterruptContext, InterruptDescriptor, InterruptEvent,
    InterruptKind, InterruptPolicy, InterruptQueue, InterruptStatus, MAX_INTERRUPT_VECTOR,
};
pub use registry::{
    Device, DeviceCall, DeviceCallBudget, DeviceCallResult, DeviceDescriptor, DeviceHandle,
    DeviceOpenRequest, DeviceProvenance, DeviceRegistry, DeviceState, DeviceStatus, DeviceUsage,
    MockDevice, RegistryDescriptor, DEVICE_CALL_FLAG_NONBLOCKING, DEVICE_CALL_FLAG_TRACE_REQUIRED,
    DEVICE_CALL_KNOWN_FLAGS, INVALID_DEVICE_ID, MAX_DEVICE_CALL_BUFFER_LEN,
};

/// Repository name.
pub const REPOSITORY: &str = "alani-devices";

/// Crate version.
pub const VERSION: &str = "0.1.0";

/// Machine-readable devices contract schema version.
pub const DEVICES_SCHEMA_VERSION: &str = "alani.devices.v1";

/// Public module names exposed by this crate.
pub const MODULES: &[&str] = &["registry", "interrupt", "dma", "classes"];

/// Feature bit for class descriptors and operation sets.
pub const DEVICES_FEATURE_CLASSES: u64 = 1 << 0;
/// Feature bit for fixed-capacity registry APIs.
pub const DEVICES_FEATURE_REGISTRY: u64 = 1 << 1;
/// Feature bit for capability-gated operation envelopes.
pub const DEVICES_FEATURE_OPERATION_ENVELOPES: u64 = 1 << 2;
/// Feature bit for interrupt handoff contracts.
pub const DEVICES_FEATURE_INTERRUPTS: u64 = 1 << 3;
/// Feature bit for DMA descriptors and policy validation.
pub const DEVICES_FEATURE_DMA: u64 = 1 << 4;
/// Feature bit for cognitive model, memory, and accelerator device classes.
pub const DEVICES_FEATURE_COGNITIVE_CLASSES: u64 = 1 << 5;
/// Feature bit for deterministic host-mode mock devices.
pub const DEVICES_FEATURE_MOCKS: u64 = 1 << 6;

/// All device feature bits known by this crate version.
pub const DEVICES_KNOWN_FEATURES: u64 = DEVICES_FEATURE_CLASSES
    | DEVICES_FEATURE_REGISTRY
    | DEVICES_FEATURE_OPERATION_ENVELOPES
    | DEVICES_FEATURE_INTERRUPTS
    | DEVICES_FEATURE_DMA
    | DEVICES_FEATURE_COGNITIVE_CLASSES
    | DEVICES_FEATURE_MOCKS;

/// Capability bit required to enumerate devices.
pub const DEVICE_RIGHT_LIST: u64 = 1 << 0;
/// Capability bit required to open a device.
pub const DEVICE_RIGHT_OPEN: u64 = 1 << 1;
/// Capability bit required to call device operations.
pub const DEVICE_RIGHT_CALL: u64 = 1 << 2;
/// Capability bit required to use DMA mappings.
pub const DEVICE_RIGHT_DMA: u64 = 1 << 3;
/// Capability bit required to bind or acknowledge interrupts.
pub const DEVICE_RIGHT_INTERRUPT: u64 = 1 << 4;
/// Capability bit required for administrative lifecycle changes.
pub const DEVICE_RIGHT_ADMIN: u64 = 1 << 5;
/// Capability bit required for cognitive model or accelerator access.
pub const DEVICE_RIGHT_COGNITION: u64 = 1 << 6;
/// Capability bit required for memory-device writes.
pub const DEVICE_RIGHT_MEMORY_WRITE: u64 = 1 << 7;
/// Capability bit required for audit-storage append operations.
pub const DEVICE_RIGHT_AUDIT_APPEND: u64 = 1 << 8;

/// All device rights known by this crate version.
pub const DEVICE_KNOWN_RIGHTS: u64 = DEVICE_RIGHT_LIST
    | DEVICE_RIGHT_OPEN
    | DEVICE_RIGHT_CALL
    | DEVICE_RIGHT_DMA
    | DEVICE_RIGHT_INTERRUPT
    | DEVICE_RIGHT_ADMIN
    | DEVICE_RIGHT_COGNITION
    | DEVICE_RIGHT_MEMORY_WRITE
    | DEVICE_RIGHT_AUDIT_APPEND;

/// Trace flag indicating the event was sampled.
pub const TRACE_FLAG_SAMPLED: u32 = 1 << 0;
/// Trace flag indicating verbose debug metadata is enabled.
pub const TRACE_FLAG_DEBUG: u32 = 1 << 1;
/// Trace flags known by this crate version.
pub const TRACE_KNOWN_FLAGS: u32 = TRACE_FLAG_SAMPLED | TRACE_FLAG_DEBUG;

/// Result alias used by device validation and host-mode APIs.
pub type DeviceResult<T> = Result<T, DeviceError>;

/// Error taxonomy for devices, operations, interrupts, and DMA.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DeviceError {
    /// Required field was omitted.
    MissingField,
    /// String metadata exceeded a documented maximum.
    FieldTooLong,
    /// Reserved flag, right, capability, or operation bits were present.
    ReservedBits,
    /// Device identifier, descriptor, or class metadata was invalid.
    InvalidDevice,
    /// Device identifier already exists.
    DuplicateDevice,
    /// Device could not be found.
    DeviceNotFound,
    /// Device lifecycle state does not allow the requested action.
    InvalidState,
    /// Required device capability was missing.
    AccessDenied,
    /// Operation is not supported by the selected device.
    UnsupportedOperation,
    /// Operation metadata or opcode is invalid.
    InvalidOperation,
    /// Buffer direction or length was invalid.
    InvalidBuffer,
    /// Buffer length exceeded a documented bound.
    BufferTooLarge,
    /// DMA descriptor or mapping metadata was invalid.
    InvalidDma,
    /// DMA memory was not pinned for the requested operation.
    DmaNotPinned,
    /// DMA policy disallowed the requested mapping.
    DmaPolicyViolation,
    /// Interrupt binding or event metadata was invalid.
    InvalidInterrupt,
    /// Interrupt event queue is full.
    InterruptQueueFull,
    /// Fixed-capacity collection is full.
    CapacityExceeded,
    /// Trace context was malformed.
    InvalidTrace,
    /// Device reported a fault.
    Faulted,
    /// Sensitive or secret payload metadata was not redacted or policy-approved.
    SensitiveData,
    /// Internal invariant failed.
    Internal,
}

impl DeviceError {
    /// Stable reason label for diagnostics and tests.
    pub const fn reason(self) -> &'static str {
        match self {
            Self::MissingField => "missing_field",
            Self::FieldTooLong => "field_too_long",
            Self::ReservedBits => "reserved_bits",
            Self::InvalidDevice => "invalid_device",
            Self::DuplicateDevice => "duplicate_device",
            Self::DeviceNotFound => "device_not_found",
            Self::InvalidState => "invalid_state",
            Self::AccessDenied => "access_denied",
            Self::UnsupportedOperation => "unsupported_operation",
            Self::InvalidOperation => "invalid_operation",
            Self::InvalidBuffer => "invalid_buffer",
            Self::BufferTooLarge => "buffer_too_large",
            Self::InvalidDma => "invalid_dma",
            Self::DmaNotPinned => "dma_not_pinned",
            Self::DmaPolicyViolation => "dma_policy_violation",
            Self::InvalidInterrupt => "invalid_interrupt",
            Self::InterruptQueueFull => "interrupt_queue_full",
            Self::CapacityExceeded => "capacity_exceeded",
            Self::InvalidTrace => "invalid_trace",
            Self::Faulted => "faulted",
            Self::SensitiveData => "sensitive_data",
            Self::Internal => "internal",
        }
    }

    /// Returns `true` when this error represents a fail-closed trust boundary.
    pub const fn is_security_relevant(self) -> bool {
        matches!(
            self,
            Self::ReservedBits
                | Self::AccessDenied
                | Self::InvalidBuffer
                | Self::BufferTooLarge
                | Self::InvalidDma
                | Self::DmaNotPinned
                | Self::DmaPolicyViolation
                | Self::InvalidInterrupt
                | Self::Faulted
                | Self::SensitiveData
        )
    }
}

/// Device rights bitset.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DeviceRights(pub u64);

impl DeviceRights {
    /// Empty rights set.
    pub const EMPTY: Self = Self(0);
    /// Device enumeration right.
    pub const LIST: Self = Self(DEVICE_RIGHT_LIST);
    /// Device open right.
    pub const OPEN: Self = Self(DEVICE_RIGHT_OPEN);
    /// Device call right.
    pub const CALL: Self = Self(DEVICE_RIGHT_CALL);
    /// DMA right.
    pub const DMA: Self = Self(DEVICE_RIGHT_DMA);
    /// Interrupt right.
    pub const INTERRUPT: Self = Self(DEVICE_RIGHT_INTERRUPT);
    /// Administrative lifecycle right.
    pub const ADMIN: Self = Self(DEVICE_RIGHT_ADMIN);
    /// Cognitive device right.
    pub const COGNITION: Self = Self(DEVICE_RIGHT_COGNITION);
    /// Memory write right.
    pub const MEMORY_WRITE: Self = Self(DEVICE_RIGHT_MEMORY_WRITE);
    /// Audit append right.
    pub const AUDIT_APPEND: Self = Self(DEVICE_RIGHT_AUDIT_APPEND);

    /// Creates a rights set from raw bits.
    pub const fn from_bits(bits: u64) -> DeviceResult<Self> {
        if bits & !DEVICE_KNOWN_RIGHTS != 0 {
            return Err(DeviceError::ReservedBits);
        }
        Ok(Self(bits))
    }

    /// Returns raw bits.
    pub const fn bits(self) -> u64 {
        self.0
    }

    /// Returns `true` when all required rights are present.
    pub const fn contains(self, required: Self) -> bool {
        (self.0 & required.0) == required.0
    }

    /// Combines two rights sets.
    pub const fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }

    /// Requires a rights set.
    pub const fn require(self, required: Self) -> DeviceResult<()> {
        if self.0 & !DEVICE_KNOWN_RIGHTS != 0 {
            return Err(DeviceError::ReservedBits);
        }
        if !self.contains(required) {
            return Err(DeviceError::AccessDenied);
        }
        Ok(())
    }
}

/// Data sensitivity classification for diagnostics and device payload metadata.
#[repr(u8)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DataClass {
    /// Public data.
    Public = 0,
    /// Operational metadata.
    Operational = 1,
    /// Sensitive data requiring redaction.
    Sensitive = 2,
    /// Secret data that must not be logged or exported.
    Secret = 3,
}

impl DataClass {
    /// Stable class label.
    pub const fn label(self) -> &'static str {
        match self {
            Self::Public => "public",
            Self::Operational => "operational",
            Self::Sensitive => "sensitive",
            Self::Secret => "secret",
        }
    }

    /// Returns `true` when broad diagnostics require redaction.
    pub const fn requires_redaction(self) -> bool {
        matches!(self, Self::Sensitive | Self::Secret)
    }
}

/// Redaction state for device call payloads and diagnostic metadata.
#[repr(u8)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RedactionState {
    /// Public data is present.
    Public = 0,
    /// Operational metadata is present.
    Operational = 1,
    /// Sensitive or secret content has been redacted.
    Redacted = 2,
    /// Sensitive content is present under an explicit policy approval.
    PolicyApprovedSensitive = 3,
    /// Sensitive or secret content is present without approval.
    UnredactedSensitive = 4,
}

impl RedactionState {
    /// Stable redaction label.
    pub const fn label(self) -> &'static str {
        match self {
            Self::Public => "public",
            Self::Operational => "operational",
            Self::Redacted => "redacted",
            Self::PolicyApprovedSensitive => "policy_approved_sensitive",
            Self::UnredactedSensitive => "unredacted_sensitive",
        }
    }
}

/// Validates redaction metadata for a classified payload.
pub const fn validate_redaction(
    data_class: DataClass,
    redaction: RedactionState,
) -> DeviceResult<()> {
    match data_class {
        DataClass::Public => {
            if matches!(redaction, RedactionState::Public) {
                Ok(())
            } else {
                Err(DeviceError::SensitiveData)
            }
        }
        DataClass::Operational => {
            if matches!(redaction, RedactionState::Operational) {
                Ok(())
            } else {
                Err(DeviceError::SensitiveData)
            }
        }
        DataClass::Sensitive => {
            if matches!(
                redaction,
                RedactionState::Redacted | RedactionState::PolicyApprovedSensitive
            ) {
                Ok(())
            } else {
                Err(DeviceError::SensitiveData)
            }
        }
        DataClass::Secret => {
            if matches!(redaction, RedactionState::Redacted) {
                Ok(())
            } else {
                Err(DeviceError::SensitiveData)
            }
        }
    }
}

/// Trace context for long-running or cross-repository device operations.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TraceContext {
    /// Trace identifier.
    pub trace_id: u64,
    /// Current span identifier.
    pub span_id: u64,
    /// Parent span identifier.
    pub parent_span_id: u64,
    /// Trace flags.
    pub flags: u32,
}

impl TraceContext {
    /// Empty trace context.
    pub const EMPTY: Self = Self {
        trace_id: 0,
        span_id: 0,
        parent_span_id: 0,
        flags: 0,
    };

    /// Creates a root trace context.
    pub const fn root(trace_id: u64, span_id: u64) -> Self {
        Self {
            trace_id,
            span_id,
            parent_span_id: 0,
            flags: TRACE_FLAG_SAMPLED,
        }
    }

    /// Creates a child trace context.
    pub const fn child(self, span_id: u64) -> Self {
        Self {
            trace_id: self.trace_id,
            span_id,
            parent_span_id: self.span_id,
            flags: self.flags,
        }
    }

    /// Returns `true` when both trace and span identifiers are present.
    pub const fn is_present(self) -> bool {
        self.trace_id != 0 && self.span_id != 0
    }

    /// Validates trace metadata.
    pub const fn validate(self) -> DeviceResult<()> {
        if self.flags & !TRACE_KNOWN_FLAGS != 0 {
            return Err(DeviceError::ReservedBits);
        }
        if self.trace_id == 0 && self.span_id == 0 && self.parent_span_id == 0 {
            return Ok(());
        }
        if self.trace_id == 0 || self.span_id == 0 {
            return Err(DeviceError::InvalidTrace);
        }
        Ok(())
    }
}

/// Implementation maturity marker for generated repository metadata.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ComponentStatus {
    /// API is present as a draft skeleton.
    Draft,
    /// API is implemented enough for host-mode experimentation.
    Experimental,
    /// API is compatible and stable.
    Stable,
}

/// Stable component identity record.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ComponentInfo {
    /// Repository name.
    pub repository: &'static str,
    /// Crate version.
    pub version: &'static str,
    /// Current implementation status.
    pub status: ComponentStatus,
}

/// Returns stable component identity metadata.
pub const fn component_info() -> ComponentInfo {
    ComponentInfo {
        repository: REPOSITORY,
        version: VERSION,
        status: ComponentStatus::Experimental,
    }
}

/// Returns the repository name.
pub const fn repository_name() -> &'static str {
    REPOSITORY
}

/// Returns public module names.
pub fn module_names() -> &'static [&'static str] {
    MODULES
}

/// Compact root view of the devices crate contract.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DevicesCatalog {
    /// Repository name.
    pub repository: &'static str,
    /// Crate version.
    pub version: &'static str,
    /// Device contract schema version.
    pub schema_version: &'static str,
    /// Feature bitmap.
    pub features: u64,
    /// Number of built-in device classes.
    pub class_count: usize,
    /// Number of known operations.
    pub operation_count: usize,
    /// Known device right bits.
    pub known_rights: u64,
}

impl DevicesCatalog {
    /// Current devices catalog.
    pub const CURRENT: Self = Self {
        repository: REPOSITORY,
        version: VERSION,
        schema_version: DEVICES_SCHEMA_VERSION,
        features: DEVICES_KNOWN_FEATURES,
        class_count: BUILTIN_DEVICE_CLASSES.len(),
        operation_count: DEVICE_OPERATION_COUNT,
        known_rights: DEVICE_KNOWN_RIGHTS,
    };

    /// Returns the current catalog.
    pub const fn current() -> Self {
        Self::CURRENT
    }

    /// Validates catalog metadata.
    pub const fn validate(self) -> DeviceResult<()> {
        if self.repository.is_empty() || self.version.is_empty() || self.schema_version.is_empty() {
            return Err(DeviceError::MissingField);
        }
        if self.features & !DEVICES_KNOWN_FEATURES != 0
            || self.known_rights & !DEVICE_KNOWN_RIGHTS != 0
        {
            return Err(DeviceError::ReservedBits);
        }
        if self.class_count == 0 || self.operation_count == 0 {
            return Err(DeviceError::InvalidDevice);
        }
        Ok(())
    }
}

/// Current devices catalog.
pub const DEVICES_CATALOG: DevicesCatalog = DevicesCatalog::CURRENT;

/// Returns the current devices catalog.
pub const fn devices_catalog() -> DevicesCatalog {
    DevicesCatalog::CURRENT
}
