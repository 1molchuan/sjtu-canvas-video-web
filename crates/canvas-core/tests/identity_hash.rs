use canvas_core::identity::{hash_stable_id, normalize_stable_id};

#[test]
fn stable_identity_hash_uses_trimmed_nfc_form() {
    let composed = hash_stable_id(" café ").expect("hash");
    let decomposed = hash_stable_id("cafe\u{301}").expect("hash");

    assert_eq!(composed, decomposed);
    assert_eq!(
        normalize_stable_id(" cafe\u{301} ").expect("normalized"),
        "café"
    );
    assert!(composed.starts_with("sha256:"));
    assert_eq!(composed.len(), 71);
}

#[test]
fn empty_stable_identity_is_rejected() {
    assert!(hash_stable_id("  ").is_err());
}
