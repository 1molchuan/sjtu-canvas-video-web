use std::net::IpAddr;

use canvas_core::video::{is_forbidden_ip, sanitize_filename_component};

#[test]
fn private_loopback_link_local_and_metadata_addresses_are_forbidden() {
    for raw in [
        "127.0.0.1",
        "10.0.0.1",
        "172.16.0.1",
        "192.168.0.1",
        "169.254.169.254",
        "100.64.0.1",
        "::1",
        "fe80::1",
        "fc00::1",
        "::ffff:127.0.0.1",
    ] {
        let address: IpAddr = raw.parse().expect("fixture IP is valid");
        assert!(is_forbidden_ip(address), "{raw} should be forbidden");
    }
    assert!(!is_forbidden_ip("1.1.1.1".parse().unwrap()));
    assert!(!is_forbidden_ip("2606:4700:4700::1111".parse().unwrap()));
}

#[test]
fn filename_component_removes_path_crlf_and_header_metacharacters() {
    let cleaned = sanitize_filename_component("../课程\r\n\"evil/../../录像");

    assert!(!cleaned.contains(".."));
    assert!(!cleaned.contains(['/', '\\', '\r', '\n', '"']));
    assert!(cleaned.contains("课程"));
    assert!(cleaned.chars().count() <= 120);
}

#[test]
fn empty_filename_component_gets_a_safe_default() {
    assert_eq!(sanitize_filename_component("../../"), "video");
}
