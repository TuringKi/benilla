//! This test has its own process so the environment profile cannot affect other suites.
#![cfg(not(target_arch = "wasm32"))]

#[test]
fn realm_and_world_builds_can_differ() {
    use benilla_protocol::{auth, client_build, messages, world_client_build};
    std::env::set_var("WOW_CLIENT_BUILD", "7272");
    std::env::set_var("WOW_WORLD_CLIENT_BUILD", "5875");
    assert_eq!(client_build(), 7272);
    assert_eq!(world_client_build(), 5875);
    let mut challenge = Vec::new();
    auth::write_logon_challenge(&mut challenge, "AB", client_build(), auth::LOCALE_TAG_EN_US)
        .unwrap();
    assert_eq!(&challenge[8..13], &[1, 18, 1, 0x68, 0x1c]);
    let session = messages::auth_session(u32::from(world_client_build()), "AB", 0, &[0; 20], &[]);
    assert_eq!(&session[..4], &[0xf3, 0x16, 0, 0]);
    std::env::remove_var("WOW_WORLD_CLIENT_BUILD");
    assert_eq!(world_client_build(), 7272);
    std::env::remove_var("WOW_CLIENT_BUILD");
    assert_eq!(client_build(), 5875);
    assert_eq!(world_client_build(), 5875);
}
