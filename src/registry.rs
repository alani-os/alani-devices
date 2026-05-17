//! Device descriptors, registry, open requests, and operation envelopes.

use crate::classes::{
    builtin_class_descriptor, DeviceCapabilities, DeviceClass, DeviceOperation, DeviceOperationSet,
    MockDeviceProfile, DEVICE_CAP_COGNITIVE, DEVICE_CAP_DMA, MAX_DEVICE_NAME_LEN,
};
use crate::dma::DmaPolicy;
use crate::interrupt::InterruptPolicy;
use crate::{
    validate_redaction, DataClass, DeviceError, DeviceResult, DeviceRights, RedactionState,
    TraceContext, DEVICE_RIGHT_CALL, DEVICE_RIGHT_OPEN,
};

/// Invalid device identifier.
pub const INVALID_DEVICE_ID: u64 = 0;

/// Maximum vendor label length.
pub const MAX_VENDOR_LABEL_LEN: usize = 64;

/// Maximum input or output buffer length for a direct device call.
pub const MAX_DEVICE_CALL_BUFFER_LEN: u64 = 64 * 1024;

/// Nonblocking operation flag.
pub const DEVICE_CALL_FLAG_NONBLOCKING: u32 = 1 << 0;
/// Flag requiring trace context to be present.
pub const DEVICE_CALL_FLAG_TRACE_REQUIRED: u32 = 1 << 1;
/// Device call flags known by this crate version.
pub const DEVICE_CALL_KNOWN_FLAGS: u32 =
    DEVICE_CALL_FLAG_NONBLOCKING | DEVICE_CALL_FLAG_TRACE_REQUIRED;

/// Module boundary descriptor.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RegistryDescriptor<'a> {
    /// Human-readable descriptor name.
    pub name: &'a str,
    /// Descriptor version.
    pub version: u32,
}

impl<'a> RegistryDescriptor<'a> {
    /// Creates a registry descriptor.
    pub const fn new(name: &'a str, version: u32) -> Self {
        Self { name, version }
    }
}

/// Device lifecycle state from discovery through removal.
#[repr(u8)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DeviceState {
    /// Device was discovered but not probed.
    Discovered = 0,
    /// Driver probe succeeded.
    Probed = 1,
    /// Descriptor is registered.
    Registered = 2,
    /// Device was configured and may be opened.
    Configured = 3,
    /// Device is open.
    Open = 4,
    /// Device is suspended.
    Suspended = 5,
    /// Device was removed.
    Removed = 6,
    /// Device faulted.
    Faulted = 7,
}

impl DeviceState {
    /// Stable state label.
    pub const fn label(self) -> &'static str {
        match self {
            Self::Discovered => "discovered",
            Self::Probed => "probed",
            Self::Registered => "registered",
            Self::Configured => "configured",
            Self::Open => "open",
            Self::Suspended => "suspended",
            Self::Removed => "removed",
            Self::Faulted => "faulted",
        }
    }

    /// Returns `true` when open is allowed.
    pub const fn allows_open(self) -> bool {
        matches!(self, Self::Registered | Self::Configured)
    }

    /// Returns `true` when device calls are allowed.
    pub const fn allows_call(self) -> bool {
        matches!(self, Self::Open)
    }
}

/// Device operation result status.
#[repr(u8)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DeviceStatus {
    /// Operation succeeded.
    Ok = 0,
    /// Device is not ready.
    NotReady = 1,
    /// Operation was deferred.
    Deferred = 2,
    /// Operation is unsupported.
    Unsupported = 3,
    /// Device is busy.
    Busy = 4,
    /// Device faulted.
    Fault = 5,
}

impl DeviceStatus {
    /// Stable status label.
    pub const fn label(self) -> &'static str {
        match self {
            Self::Ok => "ok",
            Self::NotReady => "not_ready",
            Self::Deferred => "deferred",
            Self::Unsupported => "unsupported",
            Self::Busy => "busy",
            Self::Fault => "fault",
        }
    }
}

/// Stable descriptor for one device instance.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DeviceDescriptor<'a> {
    /// Device identifier.
    pub id: u64,
    /// Device name.
    pub name: &'a str,
    /// Device class.
    pub class: DeviceClass,
    /// Vendor or provider label.
    pub vendor: &'a str,
    /// Instance number inside the class.
    pub instance: u32,
    /// Supported operations.
    pub operations: DeviceOperationSet,
    /// Declared capabilities.
    pub capabilities: DeviceCapabilities,
    /// Required rights for open.
    pub open_rights: DeviceRights,
    /// DMA policy.
    pub dma_policy: DmaPolicy,
    /// Interrupt policy.
    pub interrupt_policy: InterruptPolicy,
    /// Lifecycle state.
    pub state: DeviceState,
    /// Whether open/call should be audited.
    pub audit_required: bool,
}

impl<'a> DeviceDescriptor<'a> {
    /// Creates a descriptor with explicit operation and capability sets.
    pub const fn new(
        id: u64,
        name: &'a str,
        class: DeviceClass,
        operations: DeviceOperationSet,
        capabilities: DeviceCapabilities,
    ) -> Self {
        Self {
            id,
            name,
            class,
            vendor: "",
            instance: 0,
            operations,
            capabilities,
            open_rights: DeviceRights(DEVICE_RIGHT_OPEN | DEVICE_RIGHT_CALL),
            dma_policy: DmaPolicy::Disabled,
            interrupt_policy: InterruptPolicy::None,
            state: DeviceState::Registered,
            audit_required: true,
        }
    }

    /// Creates a descriptor from a built-in class.
    pub fn from_builtin(id: u64, name: &'a str, class: DeviceClass) -> DeviceResult<Self> {
        let Some(class_descriptor) = builtin_class_descriptor(class) else {
            return Err(DeviceError::InvalidDevice);
        };
        Ok(Self {
            id,
            name,
            class,
            vendor: "alani-mock",
            instance: 0,
            operations: class_descriptor.operations,
            capabilities: class_descriptor.capabilities,
            open_rights: class_descriptor.open_rights,
            dma_policy: if class_descriptor
                .capabilities
                .contains(DeviceCapabilities(DEVICE_CAP_DMA))
            {
                DmaPolicy::Pinned
            } else {
                DmaPolicy::Disabled
            },
            interrupt_policy: InterruptPolicy::Optional,
            state: DeviceState::Configured,
            audit_required: true,
        })
    }

    /// Sets vendor label.
    pub const fn with_vendor(mut self, vendor: &'a str) -> Self {
        self.vendor = vendor;
        self
    }

    /// Sets instance number.
    pub const fn with_instance(mut self, instance: u32) -> Self {
        self.instance = instance;
        self
    }

    /// Sets DMA policy.
    pub const fn with_dma_policy(mut self, dma_policy: DmaPolicy) -> Self {
        self.dma_policy = dma_policy;
        self
    }

    /// Sets interrupt policy.
    pub const fn with_interrupt_policy(mut self, interrupt_policy: InterruptPolicy) -> Self {
        self.interrupt_policy = interrupt_policy;
        self
    }

    /// Sets lifecycle state.
    pub const fn with_state(mut self, state: DeviceState) -> Self {
        self.state = state;
        self
    }

    /// Sets open rights.
    pub const fn with_open_rights(mut self, open_rights: DeviceRights) -> Self {
        self.open_rights = open_rights;
        self
    }

    /// Validates descriptor metadata.
    pub fn validate(self) -> DeviceResult<()> {
        if self.id == INVALID_DEVICE_ID || self.name.is_empty() {
            return Err(DeviceError::MissingField);
        }
        if self.name.len() > MAX_DEVICE_NAME_LEN || self.vendor.len() > MAX_VENDOR_LABEL_LEN {
            return Err(DeviceError::FieldTooLong);
        }
        self.operations.validate()?;
        self.capabilities.validate()?;
        self.open_rights.require(DeviceRights::OPEN)?;
        if self.operations.bits() == 0 {
            return Err(DeviceError::InvalidOperation);
        }
        if self.class.is_cognitive()
            && !self
                .capabilities
                .contains(DeviceCapabilities(DEVICE_CAP_COGNITIVE))
        {
            return Err(DeviceError::InvalidDevice);
        }
        if self.dma_policy.allows_dma()
            && !self
                .capabilities
                .contains(DeviceCapabilities(DEVICE_CAP_DMA))
        {
            return Err(DeviceError::DmaPolicyViolation);
        }
        if matches!(self.state, DeviceState::Removed | DeviceState::Faulted) {
            return Err(DeviceError::InvalidState);
        }
        Ok(())
    }
}

/// Device handle returned after an authorized open.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DeviceHandle {
    /// Device identifier.
    pub device_id: u64,
    /// Device class.
    pub class: DeviceClass,
    /// Rights granted to the handle.
    pub rights: DeviceRights,
    /// Handle generation.
    pub generation: u32,
}

impl DeviceHandle {
    /// Creates a device handle.
    pub const fn new(
        device_id: u64,
        class: DeviceClass,
        rights: DeviceRights,
        generation: u32,
    ) -> Self {
        Self {
            device_id,
            class,
            rights,
            generation,
        }
    }

    /// Validates handle metadata.
    pub const fn validate(self) -> DeviceResult<()> {
        if self.device_id == INVALID_DEVICE_ID || self.generation == 0 {
            return Err(DeviceError::InvalidDevice);
        }
        match DeviceRights::from_bits(self.rights.bits()) {
            Ok(_) => Ok(()),
            Err(error) => Err(error),
        }
    }
}

/// Request to open a device under capability control.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DeviceOpenRequest<'a> {
    /// Device identifier.
    pub device_id: u64,
    /// Principal requesting the open.
    pub principal: &'a str,
    /// Rights offered by the caller.
    pub rights: DeviceRights,
    /// Trace context.
    pub trace: TraceContext,
}

impl<'a> DeviceOpenRequest<'a> {
    /// Creates an open request.
    pub const fn new(device_id: u64, principal: &'a str, rights: DeviceRights) -> Self {
        Self {
            device_id,
            principal,
            rights,
            trace: TraceContext::EMPTY,
        }
    }

    /// Sets trace context.
    pub const fn with_trace(mut self, trace: TraceContext) -> Self {
        self.trace = trace;
        self
    }

    /// Validates open request metadata.
    pub fn validate(self) -> DeviceResult<()> {
        if self.device_id == INVALID_DEVICE_ID || self.principal.is_empty() {
            return Err(DeviceError::MissingField);
        }
        if self.principal.len() > MAX_VENDOR_LABEL_LEN {
            return Err(DeviceError::FieldTooLong);
        }
        self.rights.require(DeviceRights::OPEN)?;
        self.trace.validate()
    }
}

/// Budget carried by bounded cognitive device operations.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DeviceCallBudget {
    /// Maximum compute units the caller authorizes.
    pub max_compute_units: u64,
    /// Maximum memory bytes the caller authorizes.
    pub max_memory_bytes: u64,
    /// Optional deadline in nanoseconds from the caller's scheduling domain.
    pub deadline_ns: u64,
}

impl DeviceCallBudget {
    /// No explicit budget.
    pub const NONE: Self = Self {
        max_compute_units: 0,
        max_memory_bytes: 0,
        deadline_ns: 0,
    };

    /// Creates a budget.
    pub const fn new(max_compute_units: u64, max_memory_bytes: u64, deadline_ns: u64) -> Self {
        Self {
            max_compute_units,
            max_memory_bytes,
            deadline_ns,
        }
    }

    /// Returns `true` when at least one resource limit is set.
    pub const fn has_limit(self) -> bool {
        self.max_compute_units != 0 || self.max_memory_bytes != 0
    }

    /// Validates budget consistency.
    pub const fn validate(self) -> DeviceResult<()> {
        if self.deadline_ns != 0 && !self.has_limit() {
            return Err(DeviceError::InvalidOperation);
        }
        Ok(())
    }
}

/// Deterministic usage metadata returned by device calls.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DeviceUsage {
    /// Compute units consumed.
    pub compute_units: u64,
    /// Memory bytes consumed or touched.
    pub memory_bytes: u64,
}

impl DeviceUsage {
    /// Empty usage metadata.
    pub const EMPTY: Self = Self {
        compute_units: 0,
        memory_bytes: 0,
    };

    /// Creates usage metadata.
    pub const fn new(compute_units: u64, memory_bytes: u64) -> Self {
        Self {
            compute_units,
            memory_bytes,
        }
    }

    /// Returns `true` when usage metadata contains any non-zero value.
    pub const fn is_present(self) -> bool {
        self.compute_units != 0 || self.memory_bytes != 0
    }
}

/// Provenance metadata for a device-call result.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DeviceProvenance {
    /// Device identifier that produced the result.
    pub device_id: u64,
    /// Handle generation used for the call.
    pub generation: u32,
    /// Deterministic per-device result sequence.
    pub sequence: u64,
    /// Trace identifier associated with the call.
    pub trace_id: u64,
}

impl DeviceProvenance {
    /// Empty provenance metadata for operations where provenance is not relevant.
    pub const EMPTY: Self = Self {
        device_id: 0,
        generation: 0,
        sequence: 0,
        trace_id: 0,
    };

    /// Creates provenance metadata.
    pub const fn new(device_id: u64, generation: u32, sequence: u64, trace_id: u64) -> Self {
        Self {
            device_id,
            generation,
            sequence,
            trace_id,
        }
    }

    /// Creates provenance metadata from a call envelope and result sequence.
    pub const fn from_call(call: DeviceCall, sequence: u64) -> Self {
        Self {
            device_id: call.handle.device_id,
            generation: call.handle.generation,
            sequence,
            trace_id: call.trace.trace_id,
        }
    }

    /// Returns `true` when provenance metadata is present.
    pub const fn is_present(self) -> bool {
        self.device_id != 0 || self.generation != 0 || self.sequence != 0 || self.trace_id != 0
    }

    /// Validates provenance metadata.
    pub const fn validate(self) -> DeviceResult<()> {
        if !self.is_present() {
            return Ok(());
        }
        if self.device_id == 0 || self.generation == 0 || self.sequence == 0 {
            return Err(DeviceError::InvalidDevice);
        }
        Ok(())
    }
}

/// Common device call envelope for userspace and kernel-mediated calls.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DeviceCall {
    /// Open device handle.
    pub handle: DeviceHandle,
    /// Operation.
    pub operation: DeviceOperation,
    /// Input buffer length in bytes.
    pub input_len: u64,
    /// Output buffer length in bytes.
    pub output_len: u64,
    /// Call flags.
    pub flags: u32,
    /// Optional budget for bounded cognitive operations.
    pub budget: DeviceCallBudget,
    /// Payload data classification.
    pub data_class: DataClass,
    /// Payload redaction state.
    pub redaction: RedactionState,
    /// Trace context.
    pub trace: TraceContext,
}

impl DeviceCall {
    /// Creates a call envelope.
    pub const fn new(handle: DeviceHandle, operation: DeviceOperation) -> Self {
        Self {
            handle,
            operation,
            input_len: 0,
            output_len: 0,
            flags: 0,
            budget: DeviceCallBudget::NONE,
            data_class: DataClass::Operational,
            redaction: RedactionState::Operational,
            trace: TraceContext::EMPTY,
        }
    }

    /// Creates a call envelope from a numeric opcode.
    pub const fn from_opcode(handle: DeviceHandle, opcode: u32) -> DeviceResult<Self> {
        match DeviceOperation::from_opcode(opcode) {
            Some(operation) => Ok(Self::new(handle, operation)),
            None => Err(DeviceError::UnsupportedOperation),
        }
    }

    /// Sets input and output lengths.
    pub const fn with_buffers(mut self, input_len: u64, output_len: u64) -> Self {
        self.input_len = input_len;
        self.output_len = output_len;
        self
    }

    /// Sets flags.
    pub const fn with_flags(mut self, flags: u32) -> Self {
        self.flags = flags;
        self
    }

    /// Sets cognitive-operation budget metadata.
    pub const fn with_budget(mut self, budget: DeviceCallBudget) -> Self {
        self.budget = budget;
        self
    }

    /// Sets trace context.
    pub const fn with_trace(mut self, trace: TraceContext) -> Self {
        self.trace = trace;
        self
    }

    /// Sets data class.
    pub const fn with_data_class(mut self, data_class: DataClass) -> Self {
        self.data_class = data_class;
        self
    }

    /// Sets payload classification and redaction metadata.
    pub const fn with_data_handling(
        mut self,
        data_class: DataClass,
        redaction: RedactionState,
    ) -> Self {
        self.data_class = data_class;
        self.redaction = redaction;
        self
    }

    /// Validates call envelope metadata without checking a specific descriptor.
    pub const fn validate(self) -> DeviceResult<()> {
        match self.handle.validate() {
            Ok(()) => {}
            Err(error) => return Err(error),
        }
        if self.flags & !DEVICE_CALL_KNOWN_FLAGS != 0 {
            return Err(DeviceError::ReservedBits);
        }
        if self.input_len > MAX_DEVICE_CALL_BUFFER_LEN
            || self.output_len > MAX_DEVICE_CALL_BUFFER_LEN
        {
            return Err(DeviceError::BufferTooLarge);
        }
        if self.flags & DEVICE_CALL_FLAG_TRACE_REQUIRED != 0
            && (self.trace.trace_id == 0 || self.trace.span_id == 0)
        {
            return Err(DeviceError::InvalidTrace);
        }
        match self.budget.validate() {
            Ok(()) => {}
            Err(error) => return Err(error),
        }
        if self.operation.requires_budget() {
            if !self.budget.has_limit() {
                return Err(DeviceError::MissingField);
            }
            if !self.trace.is_present() {
                return Err(DeviceError::InvalidTrace);
            }
        }
        match validate_redaction(self.data_class, self.redaction) {
            Ok(()) => {}
            Err(error) => return Err(error),
        }
        match self.handle.rights.require(self.operation.required_rights()) {
            Ok(()) => {}
            Err(error) => return Err(error),
        }
        self.trace.validate()
    }

    /// Validates call envelope against a registered descriptor.
    pub fn validate_against(self, descriptor: DeviceDescriptor<'_>) -> DeviceResult<()> {
        self.validate()?;
        descriptor.validate()?;
        if self.handle.device_id != descriptor.id || self.handle.class != descriptor.class {
            return Err(DeviceError::InvalidDevice);
        }
        if !descriptor.state.allows_call() {
            return Err(DeviceError::InvalidState);
        }
        if !descriptor.operations.contains(self.operation) {
            return Err(DeviceError::UnsupportedOperation);
        }
        validate_operation_buffers(self.operation, self.input_len, self.output_len)?;
        if matches!(
            self.operation,
            DeviceOperation::BlockRead
                | DeviceOperation::BlockWrite
                | DeviceOperation::AcceleratorSubmitGraph
                | DeviceOperation::AcceleratorSubmitTensor
                | DeviceOperation::AcceleratorReadResult
        ) && !descriptor.dma_policy.allows_dma()
        {
            return Err(DeviceError::DmaPolicyViolation);
        }
        Ok(())
    }
}

/// Result of a device call.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DeviceCallResult {
    /// Device status.
    pub status: DeviceStatus,
    /// Output bytes written.
    pub bytes_written: u64,
    /// Usage metadata for bounded or metered operations.
    pub usage: DeviceUsage,
    /// Provenance metadata for generated or retrieved results.
    pub provenance: DeviceProvenance,
    /// Whether an audit event should be emitted.
    pub audit_required: bool,
    /// Whether bottom-half or task-context work remains.
    pub deferred: bool,
}

impl DeviceCallResult {
    /// Creates an OK call result.
    pub const fn ok(bytes_written: u64, audit_required: bool) -> Self {
        Self {
            status: DeviceStatus::Ok,
            bytes_written,
            usage: DeviceUsage::EMPTY,
            provenance: DeviceProvenance::EMPTY,
            audit_required,
            deferred: false,
        }
    }

    /// Creates an unsupported result.
    pub const fn unsupported(audit_required: bool) -> Self {
        Self {
            status: DeviceStatus::Unsupported,
            bytes_written: 0,
            usage: DeviceUsage::EMPTY,
            provenance: DeviceProvenance::EMPTY,
            audit_required,
            deferred: false,
        }
    }

    /// Sets usage metadata.
    pub const fn with_usage(mut self, usage: DeviceUsage) -> Self {
        self.usage = usage;
        self
    }

    /// Sets provenance metadata.
    pub const fn with_provenance(mut self, provenance: DeviceProvenance) -> Self {
        self.provenance = provenance;
        self
    }

    /// Validates result metadata.
    pub const fn validate(self) -> DeviceResult<()> {
        if matches!(self.status, DeviceStatus::Fault) {
            return Err(DeviceError::Faulted);
        }
        if matches!(self.status, DeviceStatus::Unsupported) {
            return Err(DeviceError::UnsupportedOperation);
        }
        if self.bytes_written > MAX_DEVICE_CALL_BUFFER_LEN {
            return Err(DeviceError::BufferTooLarge);
        }
        self.provenance.validate()
    }
}

/// Device trait implemented by concrete and mock devices.
pub trait Device {
    /// Returns the current device descriptor.
    fn descriptor(&self) -> DeviceDescriptor<'_>;

    /// Opens the device under capability control.
    fn open(&mut self, request: DeviceOpenRequest<'_>) -> DeviceResult<DeviceHandle>;

    /// Performs one device operation.
    fn call(&mut self, call: DeviceCall) -> DeviceResult<DeviceCallResult>;

    /// Closes a handle.
    fn close(&mut self, handle: DeviceHandle) -> DeviceResult<()>;
}

/// Deterministic host-mode mock device.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MockDevice<'a> {
    /// Current descriptor.
    pub descriptor: DeviceDescriptor<'a>,
    /// Mock profile.
    pub profile: MockDeviceProfile<'a>,
    /// Number of calls accepted by the mock.
    pub call_count: u64,
    generation: u32,
}

impl<'a> MockDevice<'a> {
    /// Creates a mock device.
    pub fn new(
        descriptor: DeviceDescriptor<'a>,
        profile: MockDeviceProfile<'a>,
    ) -> DeviceResult<Self> {
        descriptor.validate()?;
        profile.validate()?;
        if descriptor.class != profile.class {
            return Err(DeviceError::InvalidDevice);
        }
        Ok(Self {
            descriptor,
            profile,
            call_count: 0,
            generation: 1,
        })
    }

    /// Creates the MVK mock console.
    pub fn mock_console(id: u64) -> DeviceResult<Self> {
        Self::new(
            DeviceDescriptor::from_builtin(id, "mock-console", DeviceClass::Console)?,
            MockDeviceProfile::new("mock-console", DeviceClass::Console),
        )
    }

    /// Creates the MVK mock memory device.
    pub fn mock_memory(id: u64) -> DeviceResult<Self> {
        Self::new(
            DeviceDescriptor::from_builtin(id, "mock-memory", DeviceClass::Memory)?,
            MockDeviceProfile::new("mock-memory", DeviceClass::Memory),
        )
    }
}

impl Device for MockDevice<'_> {
    fn descriptor(&self) -> DeviceDescriptor<'_> {
        self.descriptor
    }

    fn open(&mut self, request: DeviceOpenRequest<'_>) -> DeviceResult<DeviceHandle> {
        request.validate()?;
        if request.device_id != self.descriptor.id {
            return Err(DeviceError::DeviceNotFound);
        }
        if !self.descriptor.state.allows_open() {
            return Err(DeviceError::InvalidState);
        }
        request.rights.require(self.descriptor.open_rights)?;
        self.descriptor.state = DeviceState::Open;
        Ok(DeviceHandle::new(
            self.descriptor.id,
            self.descriptor.class,
            request.rights,
            self.generation,
        ))
    }

    fn call(&mut self, call: DeviceCall) -> DeviceResult<DeviceCallResult> {
        call.validate_against(self.descriptor)?;
        self.call_count += 1;
        let bytes_written = output_bytes_for(call.operation, call.output_len);
        Ok(
            DeviceCallResult::ok(bytes_written, self.descriptor.audit_required)
                .with_usage(usage_for(call))
                .with_provenance(DeviceProvenance::from_call(call, self.call_count)),
        )
    }

    fn close(&mut self, handle: DeviceHandle) -> DeviceResult<()> {
        handle.validate()?;
        if handle.device_id != self.descriptor.id {
            return Err(DeviceError::DeviceNotFound);
        }
        self.descriptor.state = DeviceState::Configured;
        self.generation += 1;
        Ok(())
    }
}

/// Fixed-capacity registry for device descriptors.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DeviceRegistry<'a, const N: usize> {
    devices: [Option<DeviceDescriptor<'a>>; N],
    len: usize,
    next_generation: u32,
}

impl<'a, const N: usize> DeviceRegistry<'a, N> {
    /// Creates an empty registry.
    pub const fn new() -> Self {
        Self {
            devices: [None; N],
            len: 0,
            next_generation: 1,
        }
    }

    /// Returns registered device count.
    pub const fn len(self) -> usize {
        self.len
    }

    /// Returns `true` when the registry is empty.
    pub const fn is_empty(self) -> bool {
        self.len == 0
    }

    /// Registers one device descriptor.
    pub fn register(&mut self, descriptor: DeviceDescriptor<'a>) -> DeviceResult<()> {
        if self.len >= N {
            return Err(DeviceError::CapacityExceeded);
        }
        descriptor.validate()?;
        if self.contains_id(descriptor.id) {
            return Err(DeviceError::DuplicateDevice);
        }
        self.devices[self.len] = Some(descriptor);
        self.len += 1;
        Ok(())
    }

    /// Returns `true` when the device id exists.
    pub fn contains_id(self, id: u64) -> bool {
        self.find_index(id).is_some()
    }

    /// Finds a descriptor by id.
    pub fn get(self, id: u64) -> Option<DeviceDescriptor<'a>> {
        self.find_index(id).and_then(|index| self.devices[index])
    }

    /// Counts devices by class.
    pub fn count_by_class(self, class: DeviceClass) -> usize {
        let mut count = 0;
        let mut index = 0;
        while index < self.len {
            if self.devices[index].is_some_and(|descriptor| descriptor.class == class) {
                count += 1;
            }
            index += 1;
        }
        count
    }

    /// Opens a device and returns an authorized handle.
    pub fn open(&mut self, request: DeviceOpenRequest<'_>) -> DeviceResult<DeviceHandle> {
        request.validate()?;
        let Some(index) = self.find_index(request.device_id) else {
            return Err(DeviceError::DeviceNotFound);
        };
        let Some(mut descriptor) = self.devices[index] else {
            return Err(DeviceError::Internal);
        };
        descriptor.validate()?;
        if !descriptor.state.allows_open() {
            return Err(DeviceError::InvalidState);
        }
        request.rights.require(descriptor.open_rights)?;
        descriptor.state = DeviceState::Open;
        self.devices[index] = Some(descriptor);
        let handle = DeviceHandle::new(
            descriptor.id,
            descriptor.class,
            request.rights,
            self.next_generation,
        );
        self.next_generation += 1;
        Ok(handle)
    }

    /// Closes a device handle.
    pub fn close(&mut self, handle: DeviceHandle) -> DeviceResult<()> {
        handle.validate()?;
        let Some(index) = self.find_index(handle.device_id) else {
            return Err(DeviceError::DeviceNotFound);
        };
        let Some(mut descriptor) = self.devices[index] else {
            return Err(DeviceError::Internal);
        };
        if descriptor.class != handle.class {
            return Err(DeviceError::InvalidDevice);
        }
        descriptor.state = DeviceState::Configured;
        self.devices[index] = Some(descriptor);
        Ok(())
    }

    /// Validates a device call against the registered descriptor.
    pub fn validate_call(self, call: DeviceCall) -> DeviceResult<()> {
        let Some(descriptor) = self.get(call.handle.device_id) else {
            return Err(DeviceError::DeviceNotFound);
        };
        call.validate_against(descriptor)
    }

    /// Sets the lifecycle state for a registered device.
    pub fn set_state(&mut self, id: u64, state: DeviceState) -> DeviceResult<()> {
        let Some(index) = self.find_index(id) else {
            return Err(DeviceError::DeviceNotFound);
        };
        let Some(mut descriptor) = self.devices[index] else {
            return Err(DeviceError::Internal);
        };
        descriptor.state = state;
        self.devices[index] = Some(descriptor);
        Ok(())
    }

    fn find_index(self, id: u64) -> Option<usize> {
        let mut index = 0;
        while index < self.len {
            if self.devices[index].is_some_and(|descriptor| descriptor.id == id) {
                return Some(index);
            }
            index += 1;
        }
        None
    }
}

impl<'a, const N: usize> Default for DeviceRegistry<'a, N> {
    fn default() -> Self {
        Self::new()
    }
}

fn validate_operation_buffers(
    operation: DeviceOperation,
    input_len: u64,
    output_len: u64,
) -> DeviceResult<()> {
    if input_len > MAX_DEVICE_CALL_BUFFER_LEN || output_len > MAX_DEVICE_CALL_BUFFER_LEN {
        return Err(DeviceError::BufferTooLarge);
    }
    match operation {
        DeviceOperation::Read
        | DeviceOperation::Poll
        | DeviceOperation::Health
        | DeviceOperation::BlockRead
        | DeviceOperation::EntropyRead
        | DeviceOperation::ModelList
        | DeviceOperation::Infer
        | DeviceOperation::InspectMetadata
        | DeviceOperation::MemoryGet
        | DeviceOperation::MemoryQueryVector
        | DeviceOperation::MemorySnapshot
        | DeviceOperation::MemoryVerifySnapshot
        | DeviceOperation::AcceleratorReadResult
        | DeviceOperation::Receive => {
            if output_len == 0 {
                return Err(DeviceError::InvalidBuffer);
            }
        }
        DeviceOperation::Write
        | DeviceOperation::BlockWrite
        | DeviceOperation::ModelLoad
        | DeviceOperation::MemoryPut
        | DeviceOperation::AcceleratorSubmitGraph
        | DeviceOperation::AcceleratorSubmitTensor
        | DeviceOperation::AuditAppend
        | DeviceOperation::Send => {
            if input_len == 0 {
                return Err(DeviceError::InvalidBuffer);
            }
        }
        DeviceOperation::Open
        | DeviceOperation::Close
        | DeviceOperation::Reset
        | DeviceOperation::Cancel
        | DeviceOperation::MemoryCompact
        | DeviceOperation::AuditFlush
        | DeviceOperation::ModelUnload => {}
    }
    Ok(())
}

fn output_bytes_for(operation: DeviceOperation, output_len: u64) -> u64 {
    match operation {
        DeviceOperation::Write
        | DeviceOperation::BlockWrite
        | DeviceOperation::ModelLoad
        | DeviceOperation::MemoryPut
        | DeviceOperation::AcceleratorSubmitGraph
        | DeviceOperation::AcceleratorSubmitTensor
        | DeviceOperation::AuditAppend
        | DeviceOperation::Send => 0,
        _ => core::cmp::min(output_len, 32),
    }
}

fn usage_for(call: DeviceCall) -> DeviceUsage {
    if !call.operation.requires_budget() {
        return DeviceUsage::EMPTY;
    }
    let compute_units = if call.budget.max_compute_units == 0 {
        0
    } else {
        core::cmp::min(call.budget.max_compute_units, 1)
    };
    let requested_memory = call.input_len.saturating_add(call.output_len);
    let memory_bytes = if call.budget.max_memory_bytes == 0 {
        requested_memory
    } else {
        core::cmp::min(call.budget.max_memory_bytes, requested_memory)
    };
    DeviceUsage::new(compute_units, memory_bytes)
}
