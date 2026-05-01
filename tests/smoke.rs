#[test]
fn repository_identity_is_stable() {
    assert_eq!(alani_devices::repository_name(), "alani-devices");
    assert!(!alani_devices::module_names().is_empty());
}
