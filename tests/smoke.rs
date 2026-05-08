use alani_devices::{
    builtin_class_descriptor, devices_catalog, ComponentStatus, DataClass, Device, DeviceCall,
    DeviceCapabilities, DeviceClass, DeviceDescriptor, DeviceError, DeviceHandle,
    DeviceOpenRequest, DeviceOperation, DeviceOperationSet, DeviceRegistry, DeviceRights,
    DmaAddressSpace, DmaBuffer, DmaDirection, DmaMapping, DmaPolicy, InterruptAck,
    InterruptBinding, InterruptEvent, InterruptKind, InterruptPolicy, InterruptQueue,
    InterruptStatus, MockDevice, TraceContext, DEVICES_CATALOG, DEVICES_FEATURE_DMA,
    DEVICES_KNOWN_FEATURES, DEVICE_RIGHT_CALL, DEVICE_RIGHT_COGNITION, DEVICE_RIGHT_MEMORY_WRITE,
    DEVICE_RIGHT_OPEN,
};

fn rights(bits: u64) -> DeviceRights {
    DeviceRights::from_bits(bits).unwrap()
}

fn open_call_rights() -> DeviceRights {
    rights(DEVICE_RIGHT_OPEN | DEVICE_RIGHT_CALL)
}

fn cognitive_memory_rights() -> DeviceRights {
    rights(
        DEVICE_RIGHT_OPEN | DEVICE_RIGHT_CALL | DEVICE_RIGHT_COGNITION | DEVICE_RIGHT_MEMORY_WRITE,
    )
}

#[test]
fn repository_identity_and_catalog_are_stable() {
    let info = alani_devices::component_info();

    assert_eq!(alani_devices::repository_name(), "alani-devices");
    assert_eq!(info.repository, "alani-devices");
    assert_eq!(info.status, ComponentStatus::Experimental);
    assert_eq!(
        alani_devices::module_names(),
        &["registry", "interrupt", "dma", "classes"]
    );
    assert_eq!(devices_catalog(), DEVICES_CATALOG);
    assert_eq!(devices_catalog().validate(), Ok(()));
    assert_eq!(
        devices_catalog().features & DEVICES_FEATURE_DMA,
        DEVICES_FEATURE_DMA
    );
    assert_eq!(DEVICES_KNOWN_FEATURES & !devices_catalog().features, 0);
}

#[test]
fn class_descriptors_operations_and_rights_validate() {
    let console = builtin_class_descriptor(DeviceClass::Console).unwrap();
    let model = builtin_class_descriptor(DeviceClass::Model).unwrap();

    assert_eq!(console.validate(), Ok(()));
    assert!(console.operations.contains(DeviceOperation::Read));
    assert!(!console.operations.contains(DeviceOperation::Infer));
    assert_eq!(
        DeviceOperation::from_opcode(DeviceOperation::Infer.opcode()),
        Some(DeviceOperation::Infer)
    );
    assert_eq!(DeviceOperation::from_opcode(99), None);
    assert!(DeviceOperation::Infer
        .required_rights()
        .contains(DeviceRights::COGNITION));
    assert!(model.class.is_cognitive());
    assert!(model
        .capabilities
        .contains(DeviceCapabilities(alani_devices::DEVICE_CAP_COGNITIVE)));
    assert_eq!(
        DeviceOperationSet::from_bits(1 << 63),
        Err(DeviceError::ReservedBits)
    );
    assert_eq!(
        DeviceCapabilities::from_bits(1 << 63),
        Err(DeviceError::ReservedBits)
    );
}

#[test]
fn registry_prevents_duplicates_and_gates_open_and_calls() {
    let descriptor =
        DeviceDescriptor::from_builtin(1, "mock-console", DeviceClass::Console).unwrap();
    let mut registry = DeviceRegistry::<2>::new();

    assert_eq!(registry.register(descriptor), Ok(()));
    assert_eq!(
        registry.register(descriptor),
        Err(DeviceError::DuplicateDevice)
    );
    assert_eq!(registry.count_by_class(DeviceClass::Console), 1);

    let denied = DeviceOpenRequest::new(1, "agent:demo", DeviceRights::OPEN);
    assert_eq!(registry.open(denied), Err(DeviceError::AccessDenied));

    let handle = registry
        .open(
            DeviceOpenRequest::new(
                1,
                "agent:demo",
                open_call_rights().union(DeviceRights::COGNITION),
            )
            .with_trace(TraceContext::root(10, 20)),
        )
        .unwrap();
    let read = DeviceCall::new(handle, DeviceOperation::Read)
        .with_buffers(0, 16)
        .with_trace(TraceContext::root(10, 21));

    assert_eq!(registry.validate_call(read), Ok(()));
    assert_eq!(
        registry.validate_call(DeviceCall::new(handle, DeviceOperation::Infer).with_buffers(8, 16)),
        Err(DeviceError::UnsupportedOperation)
    );
    assert_eq!(
        registry.validate_call(DeviceCall::new(handle, DeviceOperation::Read)),
        Err(DeviceError::InvalidBuffer)
    );
    assert_eq!(registry.close(handle), Ok(()));
}

#[test]
fn mock_console_runs_deterministically_without_side_effects_on_unsupported_ops() {
    let mut device = MockDevice::mock_console(7).unwrap();
    let handle = device
        .open(DeviceOpenRequest::new(
            7,
            "agent:demo",
            open_call_rights().union(DeviceRights::COGNITION),
        ))
        .unwrap();
    let write = DeviceCall::new(handle, DeviceOperation::Write)
        .with_buffers(5, 0)
        .with_data_class(DataClass::Operational);
    let result = device.call(write).unwrap();

    assert_eq!(result.validate(), Ok(()));
    assert_eq!(result.bytes_written, 0);
    assert!(result.audit_required);
    assert_eq!(device.call_count, 1);
    assert_eq!(
        device.call(DeviceCall::new(handle, DeviceOperation::Infer).with_buffers(8, 16)),
        Err(DeviceError::UnsupportedOperation)
    );
    assert_eq!(device.call_count, 1);
    assert_eq!(device.close(handle), Ok(()));
}

#[test]
fn dma_buffers_require_pinning_bounds_and_policy_compatibility() {
    let handle = DeviceHandle::new(
        3,
        DeviceClass::Block,
        open_call_rights().union(DeviceRights::DMA),
        1,
    );
    let buffer = DmaBuffer::new(44, 0x1000, 4096, DmaDirection::ToDevice)
        .with_owner(9)
        .pinned()
        .sealed();
    let mapped = DmaMapping::new(handle, buffer, DmaPolicy::Pinned)
        .with_rights(DeviceRights::DMA)
        .map()
        .unwrap();

    assert_eq!(mapped.validate(), Ok(()));
    assert_eq!(mapped.unmap().unwrap().state.label(), "unmapped");

    let unpinned = DmaBuffer::new(45, 0x2000, 4096, DmaDirection::FromDevice);
    assert_eq!(unpinned.validate(), Err(DeviceError::DmaNotPinned));

    let iommu_required =
        DmaMapping::new(handle, buffer, DmaPolicy::IommuRequired).with_rights(DeviceRights::DMA);
    assert_eq!(iommu_required.map(), Err(DeviceError::DmaPolicyViolation));

    let iommu_buffer = buffer.with_address_space(DmaAddressSpace::DeviceVirtual);
    assert_eq!(
        DmaMapping::new(handle, iommu_buffer, DmaPolicy::IommuRequired)
            .with_rights(DeviceRights::DMA)
            .map()
            .unwrap()
            .state
            .label(),
        "mapped"
    );
}

#[test]
fn interrupt_bindings_defer_work_and_track_overflow() {
    let binding = InterruptBinding::new(7, 33, InterruptKind::Msi, InterruptPolicy::Required)
        .with_rights(DeviceRights::INTERRUPT);
    let ack = InterruptAck::new(binding, InterruptStatus::Deferred);
    let event = InterruptEvent::from_ack(ack, 1, TraceContext::root(90, 1));
    let mut queue = InterruptQueue::<1>::new();

    assert_eq!(binding.validate(), Ok(()));
    assert!(ack.deferred);
    assert_eq!(event.validate(), Ok(()));
    assert_eq!(queue.push(event), Ok(()));
    assert_eq!(queue.push(event), Err(DeviceError::InterruptQueueFull));
    assert_eq!(queue.overflow_count(), 1);
    assert_eq!(queue.pop().unwrap().sequence, 1);
    assert!(queue.is_empty());
    assert_eq!(
        InterruptBinding::new(7, 33, InterruptKind::Poll, InterruptPolicy::PollOnly)
            .with_rights(DeviceRights::INTERRUPT)
            .validate(),
        Err(DeviceError::InvalidInterrupt)
    );
    assert_eq!(
        InterruptAck::new(binding, InterruptStatus::Fault).validate(),
        Err(DeviceError::Faulted)
    );
}

#[test]
fn cognitive_device_classes_and_mock_memory_cover_mvk_paths() {
    let model = builtin_class_descriptor(DeviceClass::Model).unwrap();
    let memory = builtin_class_descriptor(DeviceClass::Memory).unwrap();
    let accelerator = builtin_class_descriptor(DeviceClass::Accelerator).unwrap();
    let mut registry = DeviceRegistry::<2>::new();
    let mut memory_device = MockDevice::mock_memory(8).unwrap();

    assert!(model.operations.contains(DeviceOperation::Infer));
    assert!(memory.operations.contains(DeviceOperation::MemoryPut));
    assert!(accelerator
        .operations
        .contains(DeviceOperation::AcceleratorReadResult));
    assert_eq!(registry.count_by_class(DeviceClass::Accelerator), 0);
    assert_eq!(registry.register(memory_device.descriptor()), Ok(()));

    let handle = memory_device
        .open(DeviceOpenRequest::new(
            8,
            "agent:memory",
            cognitive_memory_rights().union(DeviceRights::DMA),
        ))
        .unwrap();
    let put = DeviceCall::new(handle, DeviceOperation::MemoryPut).with_buffers(32, 0);
    let get = DeviceCall::new(handle, DeviceOperation::MemoryGet).with_buffers(0, 32);

    assert_eq!(memory_device.call(put).unwrap().bytes_written, 0);
    assert_eq!(memory_device.call(get).unwrap().bytes_written, 32);
    assert_eq!(memory_device.call_count, 2);
    assert_eq!(
        DeviceCall::from_opcode(handle, 999),
        Err(DeviceError::UnsupportedOperation)
    );
    assert!(DeviceError::DmaPolicyViolation.is_security_relevant());
}
