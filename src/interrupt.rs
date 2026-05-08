//! Interrupt binding, top-half handoff, and fixed-capacity event queues.

use crate::{DataClass, DeviceError, DeviceResult, DeviceRights, TraceContext};

/// Maximum interrupt vector accepted by skeleton validators.
pub const MAX_INTERRUPT_VECTOR: u16 = 4095;

/// Module boundary descriptor.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InterruptDescriptor<'a> {
    /// Human-readable descriptor name.
    pub name: &'a str,
    /// Descriptor version.
    pub version: u32,
}

impl<'a> InterruptDescriptor<'a> {
    /// Creates an interrupt descriptor.
    pub const fn new(name: &'a str, version: u32) -> Self {
        Self { name, version }
    }
}

/// Interrupt source kind.
#[repr(u8)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InterruptKind {
    /// Legacy line interrupt.
    Line = 0,
    /// Message-signaled interrupt.
    Msi = 1,
    /// Timer interrupt.
    Timer = 2,
    /// Poll-only pseudo-interrupt.
    Poll = 3,
}

impl InterruptKind {
    /// Stable kind label.
    pub const fn label(self) -> &'static str {
        match self {
            Self::Line => "line",
            Self::Msi => "msi",
            Self::Timer => "timer",
            Self::Poll => "poll",
        }
    }
}

/// Driver interrupt policy.
#[repr(u8)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InterruptPolicy {
    /// Device has no interrupt source.
    None = 0,
    /// Device can operate with polling or interrupts.
    Optional = 1,
    /// Device requires interrupt binding.
    Required = 2,
    /// Device must be polled and must not bind a hardware interrupt.
    PollOnly = 3,
}

impl InterruptPolicy {
    /// Stable policy label.
    pub const fn label(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Optional => "optional",
            Self::Required => "required",
            Self::PollOnly => "poll_only",
        }
    }

    /// Returns `true` when hardware binding is allowed.
    pub const fn allows_binding(self) -> bool {
        matches!(self, Self::Optional | Self::Required)
    }
}

/// Context in which interrupt-related work is performed.
#[repr(u8)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InterruptContext {
    /// Minimal top-half interrupt handler.
    TopHalf = 0,
    /// Deferred bottom-half handler.
    BottomHalf = 1,
    /// Task context.
    Task = 2,
}

impl InterruptContext {
    /// Stable context label.
    pub const fn label(self) -> &'static str {
        match self {
            Self::TopHalf => "top_half",
            Self::BottomHalf => "bottom_half",
            Self::Task => "task",
        }
    }

    /// Returns `true` when heavy work may be performed.
    pub const fn allows_heavy_work(self) -> bool {
        !matches!(self, Self::TopHalf)
    }
}

/// Interrupt binding between a device and a platform vector.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct InterruptBinding {
    /// Device identifier.
    pub device_id: u64,
    /// Interrupt vector.
    pub vector: u16,
    /// Interrupt source kind.
    pub kind: InterruptKind,
    /// Driver policy.
    pub policy: InterruptPolicy,
    /// Whether the vector is shared.
    pub shared: bool,
    /// Rights used to create the binding.
    pub rights: DeviceRights,
}

impl InterruptBinding {
    /// Creates a binding.
    pub const fn new(
        device_id: u64,
        vector: u16,
        kind: InterruptKind,
        policy: InterruptPolicy,
    ) -> Self {
        Self {
            device_id,
            vector,
            kind,
            policy,
            shared: false,
            rights: DeviceRights::EMPTY,
        }
    }

    /// Marks the binding shared.
    pub const fn shared(mut self) -> Self {
        self.shared = true;
        self
    }

    /// Sets binding rights.
    pub const fn with_rights(mut self, rights: DeviceRights) -> Self {
        self.rights = rights;
        self
    }

    /// Validates binding metadata.
    pub const fn validate(self) -> DeviceResult<()> {
        if self.device_id == 0 {
            return Err(DeviceError::InvalidDevice);
        }
        if self.vector > MAX_INTERRUPT_VECTOR {
            return Err(DeviceError::InvalidInterrupt);
        }
        if !self.policy.allows_binding() {
            return Err(DeviceError::InvalidInterrupt);
        }
        if matches!(self.kind, InterruptKind::Poll) {
            return Err(DeviceError::InvalidInterrupt);
        }
        self.rights.require(DeviceRights::INTERRUPT)
    }
}

/// Interrupt status observed by a top-half handler.
#[repr(u8)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InterruptStatus {
    /// No interrupt was pending.
    Spurious = 0,
    /// Interrupt acknowledged.
    Acknowledged = 1,
    /// Interrupt acknowledged and deferred work is required.
    Deferred = 2,
    /// Device fault was observed.
    Fault = 3,
}

impl InterruptStatus {
    /// Stable status label.
    pub const fn label(self) -> &'static str {
        match self {
            Self::Spurious => "spurious",
            Self::Acknowledged => "acknowledged",
            Self::Deferred => "deferred",
            Self::Fault => "fault",
        }
    }
}

/// Top-half acknowledgement record.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct InterruptAck {
    /// Binding that produced the acknowledgement.
    pub binding: InterruptBinding,
    /// Status.
    pub status: InterruptStatus,
    /// Whether bottom-half work should run.
    pub deferred: bool,
    /// Whether an audit event should be emitted.
    pub audit_required: bool,
}

impl InterruptAck {
    /// Creates an acknowledgement from top-half status.
    pub const fn new(binding: InterruptBinding, status: InterruptStatus) -> Self {
        Self {
            binding,
            status,
            deferred: matches!(status, InterruptStatus::Deferred | InterruptStatus::Fault),
            audit_required: matches!(status, InterruptStatus::Fault),
        }
    }

    /// Validates acknowledgement metadata.
    pub const fn validate(self) -> DeviceResult<()> {
        match self.binding.validate() {
            Ok(()) => {}
            Err(error) => return Err(error),
        }
        if matches!(self.status, InterruptStatus::Fault) {
            return Err(DeviceError::Faulted);
        }
        Ok(())
    }
}

/// Deferred interrupt event record.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct InterruptEvent {
    /// Device identifier.
    pub device_id: u64,
    /// Interrupt vector.
    pub vector: u16,
    /// Monotonic event sequence.
    pub sequence: u64,
    /// Status.
    pub status: InterruptStatus,
    /// Context where this event should be processed.
    pub context: InterruptContext,
    /// Data classification for diagnostics.
    pub data_class: DataClass,
    /// Trace context.
    pub trace: TraceContext,
}

impl InterruptEvent {
    /// Creates a deferred event from an acknowledgement.
    pub const fn from_ack(ack: InterruptAck, sequence: u64, trace: TraceContext) -> Self {
        Self {
            device_id: ack.binding.device_id,
            vector: ack.binding.vector,
            sequence,
            status: ack.status,
            context: InterruptContext::BottomHalf,
            data_class: DataClass::Operational,
            trace,
        }
    }

    /// Sets processing context.
    pub const fn with_context(mut self, context: InterruptContext) -> Self {
        self.context = context;
        self
    }

    /// Sets data class.
    pub const fn with_data_class(mut self, data_class: DataClass) -> Self {
        self.data_class = data_class;
        self
    }

    /// Validates event metadata.
    pub const fn validate(self) -> DeviceResult<()> {
        if self.device_id == 0 || self.sequence == 0 {
            return Err(DeviceError::InvalidInterrupt);
        }
        if self.vector > MAX_INTERRUPT_VECTOR {
            return Err(DeviceError::InvalidInterrupt);
        }
        if matches!(self.context, InterruptContext::TopHalf) {
            return Err(DeviceError::InvalidInterrupt);
        }
        self.trace.validate()
    }
}

/// Fixed-capacity interrupt event queue.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct InterruptQueue<const N: usize> {
    events: [Option<InterruptEvent>; N],
    head: usize,
    len: usize,
    overflow_count: u64,
}

impl<const N: usize> InterruptQueue<N> {
    /// Creates an empty queue.
    pub const fn new() -> Self {
        Self {
            events: [None; N],
            head: 0,
            len: 0,
            overflow_count: 0,
        }
    }

    /// Returns queued event count.
    pub const fn len(self) -> usize {
        self.len
    }

    /// Returns `true` when the queue is empty.
    pub const fn is_empty(self) -> bool {
        self.len == 0
    }

    /// Returns overflow count.
    pub const fn overflow_count(self) -> u64 {
        self.overflow_count
    }

    /// Pushes a deferred event.
    pub fn push(&mut self, event: InterruptEvent) -> DeviceResult<()> {
        event.validate()?;
        if self.len >= N {
            self.overflow_count += 1;
            return Err(DeviceError::InterruptQueueFull);
        }
        let index = (self.head + self.len) % N;
        self.events[index] = Some(event);
        self.len += 1;
        Ok(())
    }

    /// Pops the oldest event.
    pub fn pop(&mut self) -> Option<InterruptEvent> {
        if self.len == 0 {
            return None;
        }
        let event = self.events[self.head];
        self.events[self.head] = None;
        self.head = (self.head + 1) % N;
        self.len -= 1;
        event
    }
}

impl<const N: usize> Default for InterruptQueue<N> {
    fn default() -> Self {
        Self::new()
    }
}
