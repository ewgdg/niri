use niri_config::Config;
use smithay::desktop::Window as ServerWindow;
use wayland_client::protocol::wl_surface::WlSurface;

use super::client::ClientId;
use super::*;

struct MappedTarget {
    client_id: ClientId,
    surface: WlSurface,
    window: ServerWindow,
}

#[test]
fn matching_client_environment_prevents_initial_focus() {
    assert!(std::env::var_os("PATH").is_some());
    assert!(!window_opens_focused(
        r#"
        window-rule {
            match client-env="^PATH=" title="^target$"
            open-focused false
        }
        "#,
        false,
    ));
}

#[test]
fn non_matching_client_environment_keeps_initial_focus() {
    assert!(window_opens_focused(
        r#"
        window-rule {
            match client-env="^NIRI_TEST_MISSING_CLIENT_ENV=" title="^target$"
            open-focused false
        }
        "#,
        false,
    ));
}

#[test]
fn unknown_client_credentials_do_not_match_environment() {
    assert!(window_opens_focused(
        r#"
        window-rule {
            match client-env="^PATH=" title="^target$"
            open-focused false
        }
        "#,
        true,
    ));
}

#[test]
fn client_environment_rule_applies_after_config_reload() {
    let (mut fixture, target) = mapped_target_with_focused_peer("");
    let config = Config::parse_mem(
        r#"
        window-rule {
            match client-env="^PATH="
            focus-on-xdg-activate false
        }
        "#,
    )
    .unwrap();
    fixture.niri().config.borrow_mut().window_rules = config.window_rules;
    fixture.niri().recompute_window_rules();

    let serial = fixture.client(target.client_id).keyboard_enter_serial();
    activate_target(&mut fixture, &target, Some(serial));

    assert_eq!(target_state(&mut fixture, &target), (false, true));
}

#[test]
fn focus_disabled_marks_valid_activation_urgent_by_default() {
    let (mut fixture, target) = mapped_target_with_focused_peer(
        r#"
        window-rule {
            focus-on-xdg-activate false
        }
        "#,
    );
    let serial = fixture.client(target.client_id).keyboard_enter_serial();

    activate_target(&mut fixture, &target, Some(serial));

    assert_eq!(target_state(&mut fixture, &target), (false, true));
}

#[test]
fn urgency_disabled_ignores_focus_fallback() {
    let (mut fixture, target) = mapped_target_with_focused_peer(
        r#"
        window-rule {
            focus-on-xdg-activate false
            urgent-on-xdg-activate false
        }
        "#,
    );
    let serial = fixture.client(target.client_id).keyboard_enter_serial();

    activate_target(&mut fixture, &target, Some(serial));

    assert_eq!(target_state(&mut fixture, &target), (false, false));
}

#[test]
fn urgency_disabled_ignores_accepted_invalid_activation() {
    let (mut fixture, target) = mapped_target_with_focused_peer(
        r#"
        debug {
            honor-xdg-activation-with-invalid-serial
        }
        window-rule {
            focus-on-xdg-activate false
            urgent-on-xdg-activate false
        }
        "#,
    );

    activate_target(&mut fixture, &target, Some(0));

    assert_eq!(target_state(&mut fixture, &target), (false, false));
}

#[test]
fn urgency_disabled_ignores_serialless_activation() {
    let (mut fixture, target) = mapped_target_with_focused_peer(
        r#"
        window-rule {
            urgent-on-xdg-activate false
        }
        "#,
    );

    activate_target(&mut fixture, &target, None);

    assert_eq!(target_state(&mut fixture, &target), (false, false));
}

#[test]
fn urgency_disabled_does_not_block_valid_activation() {
    let (mut fixture, target) = mapped_target_with_focused_peer(
        r#"
        window-rule {
            urgent-on-xdg-activate false
        }
        "#,
    );
    let serial = fixture.client(target.client_id).keyboard_enter_serial();

    activate_target(&mut fixture, &target, Some(serial));

    assert_eq!(target_state(&mut fixture, &target), (true, false));
}

#[test]
fn urgency_disabled_ignores_serialless_activation_before_mapping() {
    assert_eq!(
        target_state_after_activation_before_mapping(
            r#"
            window-rule {
                open-focused false
                urgent-on-xdg-activate false
            }
            "#,
        ),
        (false, false)
    );
}

#[test]
fn serialless_activation_before_mapping_is_urgent_by_default() {
    assert_eq!(
        target_state_after_activation_before_mapping(
            r#"
            window-rule {
                open-focused false
            }
            "#,
        ),
        (false, true)
    );
}

#[test]
fn urgency_disabled_window_remains_directly_focusable() {
    let (mut fixture, target) = mapped_target_with_focused_peer(
        r#"
        window-rule {
            focus-on-xdg-activate false
            urgent-on-xdg-activate false
        }
        "#,
    );

    fixture.niri().layout.activate_window(&target.window);

    let focused_window = fixture
        .niri()
        .layout
        .focus()
        .map(|mapped| mapped.window.clone());
    assert_eq!(focused_window, Some(target.window));
}

fn window_opens_focused(config: &str, credentials_unknown: bool) -> bool {
    let mut fixture = Fixture::with_config(Config::parse_mem(config).unwrap());
    fixture.add_output(1, (1920, 1080));

    let client_id = if credentials_unknown {
        fixture.add_client_with_unknown_credentials()
    } else {
        fixture.add_client()
    };
    map_window(&mut fixture, client_id);
    let source_window = fixture
        .niri()
        .layout
        .windows()
        .next()
        .unwrap()
        .1
        .window
        .clone();

    let target = fixture.client(client_id).create_window();
    let target_surface = target.surface.clone();
    target.set_title("target");
    target.commit();
    fixture.roundtrip(client_id);
    map_existing_window(&mut fixture, client_id, &target_surface);

    let is_focused = fixture
        .niri()
        .layout
        .windows()
        .find(|(_, mapped)| mapped.window != source_window)
        .unwrap()
        .1
        .is_focused();
    is_focused
}

fn mapped_target_with_focused_peer(config: &str) -> (Fixture, MappedTarget) {
    let mut fixture = Fixture::with_config(Config::parse_mem(config).unwrap());
    fixture.add_output(1, (1920, 1080));

    let client_id = fixture.add_client();
    let surface = map_window(&mut fixture, client_id);
    let window = fixture
        .niri()
        .layout
        .windows()
        .next()
        .unwrap()
        .1
        .window
        .clone();
    map_window(&mut fixture, client_id);

    (
        fixture,
        MappedTarget {
            client_id,
            surface,
            window,
        },
    )
}

fn target_state_after_activation_before_mapping(config: &str) -> (bool, bool) {
    let mut fixture = Fixture::with_config(Config::parse_mem(config).unwrap());
    fixture.add_output(1, (1920, 1080));

    let client_id = fixture.add_client();
    map_window(&mut fixture, client_id);
    let source_window = fixture
        .niri()
        .layout
        .windows()
        .next()
        .unwrap()
        .1
        .window
        .clone();

    let target = fixture.client(client_id).create_window();
    let target_surface = target.surface.clone();
    target.commit();
    fixture.roundtrip(client_id);

    let token_result = fixture.client(client_id).request_activation_token(None);
    fixture.roundtrip(client_id);
    let token = token_result.lock().unwrap().take().unwrap();
    fixture.client(client_id).activate(token, &target_surface);
    fixture.roundtrip(client_id);

    let target = fixture.client(client_id).window(&target_surface);
    target.attach_new_buffer();
    target.ack_last_and_commit();
    fixture.double_roundtrip(client_id);

    let target = fixture
        .niri()
        .layout
        .windows()
        .find(|(_, mapped)| mapped.window != source_window)
        .unwrap()
        .1;
    (target.is_focused(), target.is_urgent())
}

fn activate_target(fixture: &mut Fixture, target: &MappedTarget, serial: Option<u32>) {
    let token_result = fixture
        .client(target.client_id)
        .request_activation_token(serial);
    fixture.roundtrip(target.client_id);
    let token = token_result.lock().unwrap().take().unwrap();
    fixture
        .client(target.client_id)
        .activate(token, &target.surface);
    fixture.double_roundtrip(target.client_id);
}

fn target_state(fixture: &mut Fixture, target: &MappedTarget) -> (bool, bool) {
    let mapped = fixture
        .niri()
        .layout
        .windows()
        .find(|(_, mapped)| mapped.window == target.window)
        .unwrap()
        .1;
    (mapped.is_focused(), mapped.is_urgent())
}

fn map_window(fixture: &mut Fixture, client_id: ClientId) -> WlSurface {
    let window = fixture.client(client_id).create_window();
    let surface = window.surface.clone();
    window.commit();
    fixture.roundtrip(client_id);
    map_existing_window(fixture, client_id, &surface);
    surface
}

fn map_existing_window(fixture: &mut Fixture, client_id: ClientId, surface: &WlSurface) {
    let window = fixture.client(client_id).window(surface);
    window.attach_new_buffer();
    window.ack_last_and_commit();
    fixture.double_roundtrip(client_id);
}
