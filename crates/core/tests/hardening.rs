//! Regression coverage for validated values and captured-state boundaries.
use mucrab_core::sftp::RemoteLocation;

#[test]
fn remote_deserialization_cannot_bypass_validation() {
    for (profile, path) in [
        ("bad profile", "/"),
        ("server", "relative"),
        ("server", "/bad\\path"),
        ("server", "/bad\npath"),
        ("server", "/\0"),
    ] {
        let input = serde_json::json!({"profile": profile, "path": path});
        assert!(
            serde_json::from_value::<RemoteLocation>(input).is_err(),
            "invalid remote location must not bypass the constructor"
        );
    }
    let input = serde_json::json!({"profile":"server", "path": format!("/{}", "x".repeat(4096))});
    assert!(serde_json::from_value::<RemoteLocation>(input).is_err());
}

#[test]
fn valid_remote_wire_values_normalize_and_round_trip() {
    for path in ["/", "/a/../café %?#", "/a//b/./c", "/../../"] {
        let location = RemoteLocation::new("server", path).unwrap();
        let wire = serde_json::to_value(&location).unwrap();
        assert_eq!(
            serde_json::from_value::<RemoteLocation>(wire).unwrap(),
            location
        );
        assert_eq!(
            RemoteLocation::parse(&location.display()).unwrap(),
            location
        );
        let mut parent = location;
        for _ in 0..8 {
            parent = parent.parent();
        }
        assert_eq!(parent, RemoteLocation::new("server", "/").unwrap());
    }
}

#[test]
fn stale_capture_has_an_actionable_error_and_copied_targets_cannot_retarget_it() {
    use mucrab_core::{CaptureChanged, EntryTarget, PropertyRequest, ViewRequest};
    let path = std::env::temp_dir().join(format!("mucrab-capture-review-{}", std::process::id()));
    std::fs::write(&path, b"captured").unwrap();
    let viewer = ViewRequest::capture(EntryTarget::Local(path.clone())).unwrap();
    let property = PropertyRequest::capture(EntryTarget::Local(path.clone())).unwrap();
    let mut copied_target = viewer.target();
    if let EntryTarget::Local(p) = &mut copied_target {
        *p = path.with_extension("different");
    }
    viewer.validate().unwrap();
    std::fs::write(&path, b"a changed source with a different length").unwrap();
    let error = property.inspect().unwrap_err();
    let stale = error
        .get_ref()
        .and_then(|e| e.downcast_ref::<CaptureChanged>())
        .expect("typed stale capture");
    assert_eq!(stale.path(), path);
    std::fs::remove_file(path).unwrap();
}
