//! DMA buffer descriptors, direction flags, and mapping policy validation.

use crate::registry::DeviceHandle;
use crate::{DeviceError, DeviceResult, DeviceRights};

/// Maximum DMA buffer length accepted by skeleton validators.
pub const MAX_DMA_BUFFER_LEN: u64 = 16 * 1024 * 1024;

/// Maximum DMA alignment accepted by skeleton validators.
pub const MAX_DMA_ALIGNMENT: u64 = 4096;

/// Module boundary descriptor.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DmaDescriptor<'a> {
    /// Human-readable descriptor name.
    pub name: &'a str,
    /// Descriptor version.
    pub version: u32,
}

impl<'a> DmaDescriptor<'a> {
    /// Creates a DMA descriptor.
    pub const fn new(name: &'a str, version: u32) -> Self {
        Self { name, version }
    }
}

/// DMA transfer direction from the CPU's perspective.
#[repr(u8)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DmaDirection {
    /// Device reads memory written by CPU.
    ToDevice = 0,
    /// Device writes memory consumed by CPU.
    FromDevice = 1,
    /// Bidirectional mapping.
    Bidirectional = 2,
}

impl DmaDirection {
    /// Stable direction label.
    pub const fn label(self) -> &'static str {
        match self {
            Self::ToDevice => "to_device",
            Self::FromDevice => "from_device",
            Self::Bidirectional => "bidirectional",
        }
    }
}

/// Address space backing a DMA buffer.
#[repr(u8)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DmaAddressSpace {
    /// Physical address chosen by the kernel or platform layer.
    Physical = 0,
    /// Shared-memory handle address space.
    SharedMemory = 1,
    /// Device-visible I/O virtual address.
    DeviceVirtual = 2,
}

impl DmaAddressSpace {
    /// Stable address-space label.
    pub const fn label(self) -> &'static str {
        match self {
            Self::Physical => "physical",
            Self::SharedMemory => "shared_memory",
            Self::DeviceVirtual => "device_virtual",
        }
    }
}

/// DMA policy declared by a device driver.
#[repr(u8)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DmaPolicy {
    /// DMA is not supported.
    Disabled = 0,
    /// DMA must use kernel-owned bounce buffers.
    BounceBuffer = 1,
    /// DMA may use pinned memory with explicit lifetime rules.
    Pinned = 2,
    /// DMA requires IOMMU-like mapping.
    IommuRequired = 3,
}

impl DmaPolicy {
    /// Stable policy label.
    pub const fn label(self) -> &'static str {
        match self {
            Self::Disabled => "disabled",
            Self::BounceBuffer => "bounce_buffer",
            Self::Pinned => "pinned",
            Self::IommuRequired => "iommu_required",
        }
    }

    /// Returns `true` when this policy supports DMA mappings.
    pub const fn allows_dma(self) -> bool {
        !matches!(self, Self::Disabled)
    }
}

/// DMA mapping lifecycle state.
#[repr(u8)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DmaState {
    /// Descriptor exists but has not been mapped.
    Prepared = 0,
    /// Mapping is active.
    Mapped = 1,
    /// Mapping was synchronized for CPU access.
    SyncedForCpu = 2,
    /// Mapping was synchronized for device access.
    SyncedForDevice = 3,
    /// Mapping was revoked.
    Unmapped = 4,
}

impl DmaState {
    /// Stable state label.
    pub const fn label(self) -> &'static str {
        match self {
            Self::Prepared => "prepared",
            Self::Mapped => "mapped",
            Self::SyncedForCpu => "synced_for_cpu",
            Self::SyncedForDevice => "synced_for_device",
            Self::Unmapped => "unmapped",
        }
    }
}

/// Bounded DMA buffer descriptor.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DmaBuffer {
    /// Buffer or shared-memory handle.
    pub handle: u64,
    /// Base address in the declared address space.
    pub address: u64,
    /// Buffer length in bytes.
    pub len: u64,
    /// Required alignment.
    pub alignment: u64,
    /// Transfer direction.
    pub direction: DmaDirection,
    /// Address space backing the address.
    pub address_space: DmaAddressSpace,
    /// Owner task or driver domain identifier.
    pub owner: u64,
    /// Whether memory was pinned before mapping.
    pub pinned: bool,
    /// Whether the buffer is sealed against concurrent mutation.
    pub sealed: bool,
}

impl DmaBuffer {
    /// Creates a DMA buffer descriptor.
    pub const fn new(handle: u64, address: u64, len: u64, direction: DmaDirection) -> Self {
        Self {
            handle,
            address,
            len,
            alignment: 8,
            direction,
            address_space: DmaAddressSpace::Physical,
            owner: 0,
            pinned: false,
            sealed: false,
        }
    }

    /// Sets alignment.
    pub const fn with_alignment(mut self, alignment: u64) -> Self {
        self.alignment = alignment;
        self
    }

    /// Sets address space.
    pub const fn with_address_space(mut self, address_space: DmaAddressSpace) -> Self {
        self.address_space = address_space;
        self
    }

    /// Sets owner.
    pub const fn with_owner(mut self, owner: u64) -> Self {
        self.owner = owner;
        self
    }

    /// Marks the buffer pinned.
    pub const fn pinned(mut self) -> Self {
        self.pinned = true;
        self
    }

    /// Marks the buffer sealed.
    pub const fn sealed(mut self) -> Self {
        self.sealed = true;
        self
    }

    /// Validates buffer bounds, alignment, and pinning state.
    pub const fn validate(self) -> DeviceResult<()> {
        if self.handle == 0 || self.address == 0 || self.len == 0 {
            return Err(DeviceError::InvalidDma);
        }
        if self.len > MAX_DMA_BUFFER_LEN {
            return Err(DeviceError::BufferTooLarge);
        }
        if self.alignment == 0
            || self.alignment > MAX_DMA_ALIGNMENT
            || (self.alignment & (self.alignment - 1)) != 0
        {
            return Err(DeviceError::InvalidDma);
        }
        if self.address & (self.alignment - 1) != 0 {
            return Err(DeviceError::InvalidDma);
        }
        if !self.pinned {
            return Err(DeviceError::DmaNotPinned);
        }
        Ok(())
    }
}

/// One active or prepared DMA mapping.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DmaMapping {
    /// Device handle authorized for the mapping.
    pub device: DeviceHandle,
    /// Buffer descriptor.
    pub buffer: DmaBuffer,
    /// Driver-declared DMA policy.
    pub policy: DmaPolicy,
    /// Mapping state.
    pub state: DmaState,
    /// Rights used to create or synchronize the mapping.
    pub rights: DeviceRights,
}

impl DmaMapping {
    /// Creates a prepared DMA mapping.
    pub const fn new(device: DeviceHandle, buffer: DmaBuffer, policy: DmaPolicy) -> Self {
        Self {
            device,
            buffer,
            policy,
            state: DmaState::Prepared,
            rights: DeviceRights::EMPTY,
        }
    }

    /// Sets rights used for the mapping.
    pub const fn with_rights(mut self, rights: DeviceRights) -> Self {
        self.rights = rights;
        self
    }

    /// Marks the mapping active.
    pub const fn map(mut self) -> DeviceResult<Self> {
        match self.validate_pre_map() {
            Ok(()) => {
                self.state = DmaState::Mapped;
                Ok(self)
            }
            Err(error) => Err(error),
        }
    }

    /// Marks the mapping unmapped.
    pub const fn unmap(mut self) -> DeviceResult<Self> {
        if !matches!(
            self.state,
            DmaState::Mapped | DmaState::SyncedForCpu | DmaState::SyncedForDevice
        ) {
            return Err(DeviceError::InvalidState);
        }
        self.state = DmaState::Unmapped;
        Ok(self)
    }

    /// Validates a mapping before activation.
    pub const fn validate_pre_map(self) -> DeviceResult<()> {
        match self.device.validate() {
            Ok(()) => {}
            Err(error) => return Err(error),
        }
        match self.buffer.validate() {
            Ok(()) => {}
            Err(error) => return Err(error),
        }
        if !self.policy.allows_dma() {
            return Err(DeviceError::DmaPolicyViolation);
        }
        match self.rights.require(DeviceRights::DMA) {
            Ok(()) => {}
            Err(error) => return Err(error),
        }
        if matches!(self.policy, DmaPolicy::IommuRequired)
            && !matches!(self.buffer.address_space, DmaAddressSpace::DeviceVirtual)
        {
            return Err(DeviceError::DmaPolicyViolation);
        }
        if matches!(self.policy, DmaPolicy::BounceBuffer) && !self.buffer.sealed {
            return Err(DeviceError::DmaPolicyViolation);
        }
        Ok(())
    }

    /// Validates the current mapping state.
    pub const fn validate(self) -> DeviceResult<()> {
        match self.validate_pre_map() {
            Ok(()) => {}
            Err(error) => return Err(error),
        }
        if matches!(self.state, DmaState::Unmapped) {
            return Err(DeviceError::InvalidState);
        }
        Ok(())
    }
}
