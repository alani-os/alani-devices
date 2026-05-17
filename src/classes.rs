//! Device classes, capabilities, operations, and host-mode mock profiles.

use crate::{
    DeviceError, DeviceResult, DeviceRights, DEVICE_RIGHT_AUDIT_APPEND, DEVICE_RIGHT_CALL,
    DEVICE_RIGHT_COGNITION, DEVICE_RIGHT_DMA, DEVICE_RIGHT_MEMORY_WRITE,
};

/// Maximum device name length.
pub const MAX_DEVICE_NAME_LEN: usize = 96;

/// Capability bit for read-style operations.
pub const DEVICE_CAP_READ: u64 = 1 << 0;
/// Capability bit for write-style operations.
pub const DEVICE_CAP_WRITE: u64 = 1 << 1;
/// Capability bit for polling.
pub const DEVICE_CAP_POLL: u64 = 1 << 2;
/// Capability bit for interrupt delivery.
pub const DEVICE_CAP_INTERRUPT: u64 = 1 << 3;
/// Capability bit for DMA.
pub const DEVICE_CAP_DMA: u64 = 1 << 4;
/// Capability bit for streaming operation.
pub const DEVICE_CAP_STREAM: u64 = 1 << 5;
/// Capability bit for block I/O.
pub const DEVICE_CAP_BLOCK_IO: u64 = 1 << 6;
/// Capability bit for entropy generation.
pub const DEVICE_CAP_ENTROPY: u64 = 1 << 7;
/// Capability bit for cognitive model, memory, or accelerator use.
pub const DEVICE_CAP_COGNITIVE: u64 = 1 << 8;
/// Capability bit for persistent memory-device behavior.
pub const DEVICE_CAP_MEMORY: u64 = 1 << 9;
/// Capability bit for accelerator behavior.
pub const DEVICE_CAP_ACCELERATOR: u64 = 1 << 10;
/// Capability bit for audit-storage behavior.
pub const DEVICE_CAP_AUDIT_STORAGE: u64 = 1 << 11;

/// All device capability bits known by this crate version.
pub const KNOWN_DEVICE_CAPABILITIES: u64 = DEVICE_CAP_READ
    | DEVICE_CAP_WRITE
    | DEVICE_CAP_POLL
    | DEVICE_CAP_INTERRUPT
    | DEVICE_CAP_DMA
    | DEVICE_CAP_STREAM
    | DEVICE_CAP_BLOCK_IO
    | DEVICE_CAP_ENTROPY
    | DEVICE_CAP_COGNITIVE
    | DEVICE_CAP_MEMORY
    | DEVICE_CAP_ACCELERATOR
    | DEVICE_CAP_AUDIT_STORAGE;

/// Number of operation enum variants represented in the skeleton.
pub const DEVICE_OPERATION_COUNT: usize = 29;

/// Module boundary descriptor.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ClassesDescriptor<'a> {
    /// Human-readable descriptor name.
    pub name: &'a str,
    /// Descriptor version.
    pub version: u32,
}

impl<'a> ClassesDescriptor<'a> {
    /// Creates a classes descriptor.
    pub const fn new(name: &'a str, version: u32) -> Self {
        Self { name, version }
    }
}

/// Device classes from the core and cognitive device specifications.
#[repr(u8)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DeviceClass {
    /// Console or terminal-like device.
    Console = 0,
    /// Timer device.
    Timer = 1,
    /// Block storage device.
    Block = 2,
    /// Entropy source.
    Entropy = 3,
    /// Simulated network device.
    NetworkSim = 4,
    /// Simulated sensor device.
    SensorSim = 5,
    /// Cognitive accelerator device.
    Accelerator = 6,
    /// Persistent or cognitive memory device.
    Memory = 7,
    /// Audit-storage device.
    AuditStorage = 8,
    /// Model execution device.
    Model = 9,
}

impl DeviceClass {
    /// Stable class label.
    pub const fn label(self) -> &'static str {
        match self {
            Self::Console => "console",
            Self::Timer => "timer",
            Self::Block => "block",
            Self::Entropy => "entropy",
            Self::NetworkSim => "network-sim",
            Self::SensorSim => "sensor-sim",
            Self::Accelerator => "accelerator",
            Self::Memory => "memory",
            Self::AuditStorage => "audit-storage",
            Self::Model => "model",
        }
    }

    /// Returns `true` for model, memory, or accelerator classes.
    pub const fn is_cognitive(self) -> bool {
        matches!(self, Self::Model | Self::Memory | Self::Accelerator)
    }
}

/// Device capability bitset.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DeviceCapabilities(pub u64);

impl DeviceCapabilities {
    /// Empty capability set.
    pub const EMPTY: Self = Self(0);
    /// Console defaults.
    pub const CONSOLE: Self = Self(DEVICE_CAP_READ | DEVICE_CAP_WRITE | DEVICE_CAP_POLL);
    /// Timer defaults.
    pub const TIMER: Self = Self(DEVICE_CAP_POLL | DEVICE_CAP_INTERRUPT);
    /// Block-device defaults.
    pub const BLOCK: Self = Self(
        DEVICE_CAP_READ | DEVICE_CAP_WRITE | DEVICE_CAP_BLOCK_IO | DEVICE_CAP_DMA | DEVICE_CAP_POLL,
    );
    /// Entropy-source defaults.
    pub const ENTROPY: Self = Self(DEVICE_CAP_READ | DEVICE_CAP_ENTROPY);
    /// Simulated network defaults.
    pub const NETWORK_SIM: Self = Self(
        DEVICE_CAP_READ
            | DEVICE_CAP_WRITE
            | DEVICE_CAP_POLL
            | DEVICE_CAP_INTERRUPT
            | DEVICE_CAP_STREAM,
    );
    /// Simulated sensor defaults.
    pub const SENSOR_SIM: Self = Self(DEVICE_CAP_READ | DEVICE_CAP_POLL | DEVICE_CAP_INTERRUPT);
    /// Model device defaults.
    pub const MODEL: Self = Self(DEVICE_CAP_READ | DEVICE_CAP_WRITE | DEVICE_CAP_COGNITIVE);
    /// Memory device defaults.
    pub const MEMORY: Self = Self(
        DEVICE_CAP_READ
            | DEVICE_CAP_WRITE
            | DEVICE_CAP_COGNITIVE
            | DEVICE_CAP_MEMORY
            | DEVICE_CAP_DMA,
    );
    /// Accelerator device defaults.
    pub const ACCELERATOR: Self = Self(
        DEVICE_CAP_READ
            | DEVICE_CAP_WRITE
            | DEVICE_CAP_COGNITIVE
            | DEVICE_CAP_ACCELERATOR
            | DEVICE_CAP_DMA
            | DEVICE_CAP_INTERRUPT,
    );
    /// Audit-storage defaults.
    pub const AUDIT_STORAGE: Self =
        Self(DEVICE_CAP_WRITE | DEVICE_CAP_AUDIT_STORAGE | DEVICE_CAP_DMA);

    /// Creates a capability set from raw bits.
    pub const fn from_bits(bits: u64) -> DeviceResult<Self> {
        if bits & !KNOWN_DEVICE_CAPABILITIES != 0 {
            return Err(DeviceError::ReservedBits);
        }
        Ok(Self(bits))
    }

    /// Returns raw bits.
    pub const fn bits(self) -> u64 {
        self.0
    }

    /// Returns `true` when all requested capabilities are present.
    pub const fn contains(self, required: Self) -> bool {
        (self.0 & required.0) == required.0
    }

    /// Combines two capability sets.
    pub const fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }

    /// Validates reserved capability bits.
    pub const fn validate(self) -> DeviceResult<()> {
        if self.0 & !KNOWN_DEVICE_CAPABILITIES != 0 {
            return Err(DeviceError::ReservedBits);
        }
        Ok(())
    }
}

/// Device operation opcodes used by registry and syscall envelopes.
#[repr(u8)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DeviceOperation {
    /// Open a device.
    Open = 0,
    /// Close a device.
    Close = 1,
    /// Read bytes or records.
    Read = 2,
    /// Write bytes or records.
    Write = 3,
    /// Poll for readiness.
    Poll = 4,
    /// Reset device.
    Reset = 5,
    /// Read health metadata.
    Health = 6,
    /// Block read.
    BlockRead = 7,
    /// Block write.
    BlockWrite = 8,
    /// Read entropy.
    EntropyRead = 9,
    /// List model metadata.
    ModelList = 10,
    /// Load a model.
    ModelLoad = 11,
    /// Unload a model.
    ModelUnload = 12,
    /// Run inference.
    Infer = 13,
    /// Cancel a cognitive operation.
    Cancel = 14,
    /// Inspect model or device metadata.
    InspectMetadata = 15,
    /// Put a memory record.
    MemoryPut = 16,
    /// Get a memory record.
    MemoryGet = 17,
    /// Query vector memory.
    MemoryQueryVector = 18,
    /// Compact memory.
    MemoryCompact = 19,
    /// Snapshot memory.
    MemorySnapshot = 20,
    /// Verify memory snapshot.
    MemoryVerifySnapshot = 21,
    /// Submit accelerator graph.
    AcceleratorSubmitGraph = 22,
    /// Submit accelerator tensor.
    AcceleratorSubmitTensor = 23,
    /// Read accelerator result.
    AcceleratorReadResult = 24,
    /// Append audit storage data.
    AuditAppend = 25,
    /// Flush audit storage data.
    AuditFlush = 26,
    /// Receive simulated packet or event.
    Receive = 27,
    /// Send simulated packet or event.
    Send = 28,
}

impl DeviceOperation {
    /// Stable operation label.
    pub const fn label(self) -> &'static str {
        match self {
            Self::Open => "open",
            Self::Close => "close",
            Self::Read => "read",
            Self::Write => "write",
            Self::Poll => "poll",
            Self::Reset => "reset",
            Self::Health => "health",
            Self::BlockRead => "block_read",
            Self::BlockWrite => "block_write",
            Self::EntropyRead => "entropy_read",
            Self::ModelList => "list_models",
            Self::ModelLoad => "load_model",
            Self::ModelUnload => "unload_model",
            Self::Infer => "infer",
            Self::Cancel => "cancel",
            Self::InspectMetadata => "inspect_metadata",
            Self::MemoryPut => "put_record",
            Self::MemoryGet => "get_record",
            Self::MemoryQueryVector => "query_vector",
            Self::MemoryCompact => "compact",
            Self::MemorySnapshot => "snapshot",
            Self::MemoryVerifySnapshot => "verify_snapshot",
            Self::AcceleratorSubmitGraph => "submit_graph",
            Self::AcceleratorSubmitTensor => "submit_tensor",
            Self::AcceleratorReadResult => "read_result",
            Self::AuditAppend => "audit_append",
            Self::AuditFlush => "audit_flush",
            Self::Receive => "receive",
            Self::Send => "send",
        }
    }

    /// Stable numeric opcode.
    pub const fn opcode(self) -> u32 {
        self as u32
    }

    /// Parses a stable numeric opcode.
    pub const fn from_opcode(opcode: u32) -> Option<Self> {
        match opcode {
            0 => Some(Self::Open),
            1 => Some(Self::Close),
            2 => Some(Self::Read),
            3 => Some(Self::Write),
            4 => Some(Self::Poll),
            5 => Some(Self::Reset),
            6 => Some(Self::Health),
            7 => Some(Self::BlockRead),
            8 => Some(Self::BlockWrite),
            9 => Some(Self::EntropyRead),
            10 => Some(Self::ModelList),
            11 => Some(Self::ModelLoad),
            12 => Some(Self::ModelUnload),
            13 => Some(Self::Infer),
            14 => Some(Self::Cancel),
            15 => Some(Self::InspectMetadata),
            16 => Some(Self::MemoryPut),
            17 => Some(Self::MemoryGet),
            18 => Some(Self::MemoryQueryVector),
            19 => Some(Self::MemoryCompact),
            20 => Some(Self::MemorySnapshot),
            21 => Some(Self::MemoryVerifySnapshot),
            22 => Some(Self::AcceleratorSubmitGraph),
            23 => Some(Self::AcceleratorSubmitTensor),
            24 => Some(Self::AcceleratorReadResult),
            25 => Some(Self::AuditAppend),
            26 => Some(Self::AuditFlush),
            27 => Some(Self::Receive),
            28 => Some(Self::Send),
            _ => None,
        }
    }

    /// Required rights for the operation.
    pub const fn required_rights(self) -> DeviceRights {
        match self {
            Self::Open => DeviceRights::OPEN,
            Self::Close | Self::Poll | Self::Health | Self::Read => DeviceRights::CALL,
            Self::Write | Self::Reset | Self::Receive | Self::Send => DeviceRights::CALL,
            Self::BlockRead | Self::BlockWrite => {
                DeviceRights(DEVICE_RIGHT_CALL | DEVICE_RIGHT_DMA)
            }
            Self::EntropyRead => DeviceRights::CALL,
            Self::ModelList | Self::InspectMetadata => {
                DeviceRights(DEVICE_RIGHT_CALL | DEVICE_RIGHT_COGNITION)
            }
            Self::ModelLoad | Self::ModelUnload | Self::Infer | Self::Cancel => {
                DeviceRights(DEVICE_RIGHT_CALL | DEVICE_RIGHT_COGNITION)
            }
            Self::MemoryPut => {
                DeviceRights(DEVICE_RIGHT_CALL | DEVICE_RIGHT_COGNITION | DEVICE_RIGHT_MEMORY_WRITE)
            }
            Self::MemoryGet
            | Self::MemoryQueryVector
            | Self::MemoryCompact
            | Self::MemorySnapshot
            | Self::MemoryVerifySnapshot => {
                DeviceRights(DEVICE_RIGHT_CALL | DEVICE_RIGHT_COGNITION)
            }
            Self::AcceleratorSubmitGraph
            | Self::AcceleratorSubmitTensor
            | Self::AcceleratorReadResult => {
                DeviceRights(DEVICE_RIGHT_CALL | DEVICE_RIGHT_COGNITION | DEVICE_RIGHT_DMA)
            }
            Self::AuditAppend | Self::AuditFlush => {
                DeviceRights(DEVICE_RIGHT_CALL | DEVICE_RIGHT_AUDIT_APPEND)
            }
        }
    }

    /// Returns `true` when the operation belongs to a cognitive device path.
    pub const fn is_cognitive(self) -> bool {
        matches!(
            self,
            Self::ModelList
                | Self::ModelLoad
                | Self::ModelUnload
                | Self::Infer
                | Self::Cancel
                | Self::InspectMetadata
                | Self::MemoryPut
                | Self::MemoryGet
                | Self::MemoryQueryVector
                | Self::MemoryCompact
                | Self::MemorySnapshot
                | Self::MemoryVerifySnapshot
                | Self::AcceleratorSubmitGraph
                | Self::AcceleratorSubmitTensor
                | Self::AcceleratorReadResult
        )
    }

    /// Returns `true` when the operation must carry explicit budget metadata.
    pub const fn requires_budget(self) -> bool {
        self.is_cognitive()
    }

    /// Returns `true` when the operation can be invoked in interrupt top-half context.
    pub const fn allows_interrupt_context(self) -> bool {
        matches!(self, Self::Poll | Self::Health)
    }
}

/// Device operation bitset.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DeviceOperationSet(pub u64);

impl DeviceOperationSet {
    /// Empty operation set.
    pub const EMPTY: Self = Self(0);

    /// Creates a set from raw bits.
    pub const fn from_bits(bits: u64) -> DeviceResult<Self> {
        if bits & !Self::known_bits() != 0 {
            return Err(DeviceError::ReservedBits);
        }
        Ok(Self(bits))
    }

    /// Returns all known operation bits.
    pub const fn known_bits() -> u64 {
        (1_u64 << DEVICE_OPERATION_COUNT) - 1
    }

    /// Returns raw bits.
    pub const fn bits(self) -> u64 {
        self.0
    }

    /// Creates a one-operation set.
    pub const fn one(operation: DeviceOperation) -> Self {
        Self(1_u64 << operation.opcode())
    }

    /// Combines two operation sets.
    pub const fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }

    /// Returns `true` when the operation is supported.
    pub const fn contains(self, operation: DeviceOperation) -> bool {
        (self.0 & (1_u64 << operation.opcode())) != 0
    }

    /// Validates reserved operation bits.
    pub const fn validate(self) -> DeviceResult<()> {
        if self.0 & !Self::known_bits() != 0 {
            return Err(DeviceError::ReservedBits);
        }
        Ok(())
    }
}

/// Built-in class descriptor.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DeviceClassDescriptor {
    /// Device class.
    pub class: DeviceClass,
    /// Stable class name.
    pub name: &'static str,
    /// Supported operations for the class.
    pub operations: DeviceOperationSet,
    /// Default capabilities for the class.
    pub capabilities: DeviceCapabilities,
    /// Required rights for opening the class.
    pub open_rights: DeviceRights,
    /// Whether mock instances are expected for MVK host tests.
    pub mock_supported: bool,
}

impl DeviceClassDescriptor {
    /// Validates class descriptor metadata.
    pub const fn validate(self) -> DeviceResult<()> {
        if self.name.is_empty() {
            return Err(DeviceError::MissingField);
        }
        if self.name.len() > MAX_DEVICE_NAME_LEN {
            return Err(DeviceError::FieldTooLong);
        }
        match self.operations.validate() {
            Ok(()) => match self.capabilities.validate() {
                Ok(()) => self.open_rights.require(DeviceRights::OPEN),
                Err(error) => Err(error),
            },
            Err(error) => Err(error),
        }
    }
}

const fn ops(bits: &[DeviceOperation]) -> DeviceOperationSet {
    let mut set = DeviceOperationSet::EMPTY;
    let mut index = 0;
    while index < bits.len() {
        set = set.union(DeviceOperationSet::one(bits[index]));
        index += 1;
    }
    set
}

/// Built-in device classes from Docs 13 and 14.
pub const BUILTIN_DEVICE_CLASSES: [DeviceClassDescriptor; 10] = [
    DeviceClassDescriptor {
        class: DeviceClass::Console,
        name: "console",
        operations: ops(&[
            DeviceOperation::Open,
            DeviceOperation::Close,
            DeviceOperation::Read,
            DeviceOperation::Write,
            DeviceOperation::Poll,
            DeviceOperation::Health,
        ]),
        capabilities: DeviceCapabilities::CONSOLE,
        open_rights: DeviceRights(crate::DEVICE_RIGHT_OPEN | crate::DEVICE_RIGHT_CALL),
        mock_supported: true,
    },
    DeviceClassDescriptor {
        class: DeviceClass::Timer,
        name: "timer",
        operations: ops(&[
            DeviceOperation::Open,
            DeviceOperation::Close,
            DeviceOperation::Poll,
            DeviceOperation::Health,
        ]),
        capabilities: DeviceCapabilities::TIMER,
        open_rights: DeviceRights(crate::DEVICE_RIGHT_OPEN | crate::DEVICE_RIGHT_CALL),
        mock_supported: true,
    },
    DeviceClassDescriptor {
        class: DeviceClass::Block,
        name: "block",
        operations: ops(&[
            DeviceOperation::Open,
            DeviceOperation::Close,
            DeviceOperation::BlockRead,
            DeviceOperation::BlockWrite,
            DeviceOperation::Health,
        ]),
        capabilities: DeviceCapabilities::BLOCK,
        open_rights: DeviceRights(
            crate::DEVICE_RIGHT_OPEN | crate::DEVICE_RIGHT_CALL | crate::DEVICE_RIGHT_DMA,
        ),
        mock_supported: true,
    },
    DeviceClassDescriptor {
        class: DeviceClass::Entropy,
        name: "entropy",
        operations: ops(&[
            DeviceOperation::Open,
            DeviceOperation::Close,
            DeviceOperation::EntropyRead,
            DeviceOperation::Health,
        ]),
        capabilities: DeviceCapabilities::ENTROPY,
        open_rights: DeviceRights(crate::DEVICE_RIGHT_OPEN | crate::DEVICE_RIGHT_CALL),
        mock_supported: true,
    },
    DeviceClassDescriptor {
        class: DeviceClass::NetworkSim,
        name: "network-sim",
        operations: ops(&[
            DeviceOperation::Open,
            DeviceOperation::Close,
            DeviceOperation::Receive,
            DeviceOperation::Send,
            DeviceOperation::Poll,
            DeviceOperation::Health,
        ]),
        capabilities: DeviceCapabilities::NETWORK_SIM,
        open_rights: DeviceRights(crate::DEVICE_RIGHT_OPEN | crate::DEVICE_RIGHT_CALL),
        mock_supported: true,
    },
    DeviceClassDescriptor {
        class: DeviceClass::SensorSim,
        name: "sensor-sim",
        operations: ops(&[
            DeviceOperation::Open,
            DeviceOperation::Close,
            DeviceOperation::Read,
            DeviceOperation::Poll,
            DeviceOperation::Health,
        ]),
        capabilities: DeviceCapabilities::SENSOR_SIM,
        open_rights: DeviceRights(crate::DEVICE_RIGHT_OPEN | crate::DEVICE_RIGHT_CALL),
        mock_supported: true,
    },
    DeviceClassDescriptor {
        class: DeviceClass::Accelerator,
        name: "accelerator",
        operations: ops(&[
            DeviceOperation::Open,
            DeviceOperation::Close,
            DeviceOperation::AcceleratorSubmitGraph,
            DeviceOperation::AcceleratorSubmitTensor,
            DeviceOperation::AcceleratorReadResult,
            DeviceOperation::Reset,
            DeviceOperation::Health,
        ]),
        capabilities: DeviceCapabilities::ACCELERATOR,
        open_rights: DeviceRights(
            crate::DEVICE_RIGHT_OPEN
                | crate::DEVICE_RIGHT_CALL
                | crate::DEVICE_RIGHT_COGNITION
                | crate::DEVICE_RIGHT_DMA,
        ),
        mock_supported: true,
    },
    DeviceClassDescriptor {
        class: DeviceClass::Memory,
        name: "memory",
        operations: ops(&[
            DeviceOperation::Open,
            DeviceOperation::Close,
            DeviceOperation::MemoryPut,
            DeviceOperation::MemoryGet,
            DeviceOperation::MemoryQueryVector,
            DeviceOperation::MemoryCompact,
            DeviceOperation::MemorySnapshot,
            DeviceOperation::MemoryVerifySnapshot,
        ]),
        capabilities: DeviceCapabilities::MEMORY,
        open_rights: DeviceRights(
            crate::DEVICE_RIGHT_OPEN
                | crate::DEVICE_RIGHT_CALL
                | crate::DEVICE_RIGHT_COGNITION
                | crate::DEVICE_RIGHT_MEMORY_WRITE,
        ),
        mock_supported: true,
    },
    DeviceClassDescriptor {
        class: DeviceClass::AuditStorage,
        name: "audit-storage",
        operations: ops(&[
            DeviceOperation::Open,
            DeviceOperation::Close,
            DeviceOperation::AuditAppend,
            DeviceOperation::AuditFlush,
            DeviceOperation::Health,
        ]),
        capabilities: DeviceCapabilities::AUDIT_STORAGE,
        open_rights: DeviceRights(
            crate::DEVICE_RIGHT_OPEN | crate::DEVICE_RIGHT_CALL | crate::DEVICE_RIGHT_AUDIT_APPEND,
        ),
        mock_supported: true,
    },
    DeviceClassDescriptor {
        class: DeviceClass::Model,
        name: "model",
        operations: ops(&[
            DeviceOperation::Open,
            DeviceOperation::Close,
            DeviceOperation::ModelList,
            DeviceOperation::ModelLoad,
            DeviceOperation::ModelUnload,
            DeviceOperation::Infer,
            DeviceOperation::Cancel,
            DeviceOperation::InspectMetadata,
        ]),
        capabilities: DeviceCapabilities::MODEL,
        open_rights: DeviceRights(
            crate::DEVICE_RIGHT_OPEN | crate::DEVICE_RIGHT_CALL | crate::DEVICE_RIGHT_COGNITION,
        ),
        mock_supported: true,
    },
];

/// Finds the built-in descriptor for a device class.
pub fn builtin_class_descriptor(class: DeviceClass) -> Option<DeviceClassDescriptor> {
    BUILTIN_DEVICE_CLASSES
        .iter()
        .copied()
        .find(|descriptor| descriptor.class == class)
}

/// Deterministic host-mode mock profile for a device class.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MockDeviceProfile<'a> {
    /// Stable mock device name.
    pub name: &'a str,
    /// Device class.
    pub class: DeviceClass,
    /// Whether hardware-specific behavior is simulated.
    pub simulated: bool,
    /// Deterministic response status code.
    pub deterministic_status: u32,
}

impl<'a> MockDeviceProfile<'a> {
    /// Creates a mock profile.
    pub const fn new(name: &'a str, class: DeviceClass) -> Self {
        Self {
            name,
            class,
            simulated: true,
            deterministic_status: 0,
        }
    }

    /// Validates mock profile metadata.
    pub fn validate(self) -> DeviceResult<()> {
        if self.name.is_empty() {
            return Err(DeviceError::MissingField);
        }
        if self.name.len() > MAX_DEVICE_NAME_LEN {
            return Err(DeviceError::FieldTooLong);
        }
        let Some(descriptor) = builtin_class_descriptor(self.class) else {
            return Err(DeviceError::InvalidDevice);
        };
        if !descriptor.mock_supported || !self.simulated {
            return Err(DeviceError::InvalidDevice);
        }
        Ok(())
    }
}
