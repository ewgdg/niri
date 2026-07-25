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
fn never_ignores_serialless_activation_for_mapped_window() {
    let (mut fixture, target) = mapped_target_with_focused_peer(
        r#"
        window-rule {
            xdg-activation "never"
        }
        "#,
    );

    activate_target(&mut fixture, &target, None);

    assert_eq!(target_state(&mut fixture, &target), (false, false));
}

#[test]
fn urgent_downgrades_accepted_invalid_activation_for_mapped_window() {
    let (mut fixture, target) = mapped_target_with_focused_peer(
        r#"
        debug {
            honor-xdg-activation-with-invalid-serial
        }
        window-rule {
            xdg-activation "urgent"
        }
        "#,
    );

    activate_target(&mut fixture, &target, Some(0));

    assert_eq!(target_state(&mut fixture, &target), (false, true));
}

#[test]
fn valid_only_ignores_accepted_invalid_activation() {
    let (mut fixture, target) = mapped_target_with_focused_peer(
        r#"
        debug {
            honor-xdg-activation-with-invalid-serial
        }
        window-rule {
            xdg-activation "valid-only"
        }
        "#,
    );

    activate_target(&mut fixture, &target, Some(0));

    assert_eq!(target_state(&mut fixture, &target), (false, false));
}

#[test]
fn valid_or_urgent_downgrades_invalid_activation_without_global_override() {
    let (mut fixture, target) = mapped_target_with_focused_peer(
        r#"
        window-rule {
            xdg-activation "valid-or-urgent"
        }
        "#,
    );

    activate_target(&mut fixture, &target, Some(0));

    assert_eq!(target_state(&mut fixture, &target), (false, true));
}

#[test]
fn urgent_preserves_attention_for_activation_before_mapping() {
    let config = Config::parse_mem(
        r#"
        window-rule {
            open-focused false
            xdg-activation "urgent"
        }
        "#,
    )
    .unwrap();
    let mut fixture = Fixture::with_config(config);
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
    assert_eq!((target.is_focused(), target.is_urgent()), (false, true));
}

#[test]
fn valid_only_accepts_valid_activation() {
    let (mut fixture, target) = mapped_target_with_focused_peer(
        r#"
        window-rule {
            xdg-activation "valid-only"
        }
        "#,
    );
    let serial = fixture.client(target.client_id).keyboard_enter_serial();

    activate_target(&mut fixture, &target, Some(serial));

    assert_eq!(target_state(&mut fixture, &target), (true, false));
}

#[test]
fn later_target_rule_can_restore_default_policy() {
    let config = Config::parse_mem(
        r#"
        debug {
            honor-xdg-activation-with-invalid-serial
        }
        window-rule {
            match app-id="^automation$"
            xdg-activation "never"
        }
        window-rule {
            match app-id="^automation$" title="^Allowed$"
            xdg-activation "default"
        }
        "#,
    )
    .unwrap();
    let mut fixture = Fixture::with_config(config);
    fixture.add_output(1, (1920, 1080));

    let client_id = fixture.add_client();
    let window = fixture.client(client_id).create_window();
    window.set_app_id("automation");
    window.set_title("Allowed");
    let surface = window.surface.clone();
    window.commit();
    fixture.roundtrip(client_id);
    map_existing_window(&mut fixture, client_id, &surface);
    let target = MappedTarget {
        client_id,
        surface,
        window: fixture
            .niri()
            .layout
            .windows()
            .next()
            .unwrap()
            .1
            .window
            .clone(),
    };
    map_window(&mut fixture, client_id);

    activate_target(&mut fixture, &target, Some(0));

    assert!(target_state(&mut fixture, &target).0);
}

#[test]
fn config_reload_updates_policy_for_mapped_target() {
    let (mut fixture, target) = mapped_target_with_focused_peer(
        r#"
        window-rule {
            xdg-activation "never"
        }
        "#,
    );
    let updated_rules = Config::parse_mem(
        r#"
        window-rule {
            xdg-activation "urgent"
        }
        "#,
    )
    .unwrap()
    .window_rules;
    fixture.niri().config.borrow_mut().window_rules = updated_rules;
    fixture.niri().recompute_window_rules();

    activate_target(&mut fixture, &target, None);

    assert_eq!(target_state(&mut fixture, &target), (false, true));
}

#[test]
fn protected_window_remains_directly_focusable() {
    let (mut fixture, target) = mapped_target_with_focused_peer(
        r#"
        window-rule {
            xdg-activation "never"
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

#[test]
fn default_rejects_invalid_activation_without_global_override() {
    let (mut fixture, target) = mapped_target_with_focused_peer(
        r#"
        window-rule {
            xdg-activation "default"
        }
        "#,
    );

    activate_target(&mut fixture, &target, Some(0));

    assert_eq!(target_state(&mut fixture, &target), (false, false));
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
