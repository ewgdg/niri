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
            on-xdg-activate "set-urgent"
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
fn mapped_activation_obeys_policy_and_serial() {
    for (policy, valid, serialless) in [
        (None, (true, false), (false, true)),
        (Some("ignore"), (false, false), (false, false)),
        (Some("set-urgent"), (false, true), (false, true)),
        (Some("focus"), (true, false), (true, false)),
    ] {
        let config = activation_config(policy);
        for (with_serial, expected) in [(true, valid), (false, serialless)] {
            let (mut fixture, target) = mapped_target_with_focused_peer(&config);
            let serial =
                with_serial.then(|| fixture.client(target.client_id).keyboard_enter_serial());

            activate_target(&mut fixture, &target, serial);

            assert_eq!(
                target_state(&mut fixture, &target),
                expected,
                "policy {policy:?}, with_serial {with_serial}",
            );
        }
    }
}

#[test]
fn explicit_policy_does_not_accept_invalid_serial() {
    for policy in ["ignore", "set-urgent", "focus"] {
        let (mut fixture, target) =
            mapped_target_with_focused_peer(&activation_config(Some(policy)));

        activate_target(&mut fixture, &target, Some(0));

        assert_eq!(
            target_state(&mut fixture, &target),
            (false, false),
            "{policy}"
        );
    }
}

#[test]
fn explicit_policy_applies_to_accepted_invalid_serial() {
    for (policy, expected) in [
        ("ignore", (false, false)),
        ("set-urgent", (false, true)),
        ("focus", (true, false)),
    ] {
        let config = format!(
            "debug {{ honor-xdg-activation-with-invalid-serial; }}\n{}",
            activation_config(Some(policy)),
        );
        let (mut fixture, target) = mapped_target_with_focused_peer(&config);

        activate_target(&mut fixture, &target, Some(0));

        assert_eq!(target_state(&mut fixture, &target), expected, "{policy}");
    }
}

#[test]
fn activation_before_mapping_obeys_policy_and_serial() {
    for (policy, valid, serialless) in [
        (None, (true, false), (false, true)),
        (Some("ignore"), (false, false), (false, false)),
        (Some("set-urgent"), (false, true), (false, true)),
        (Some("focus"), (true, false), (true, false)),
    ] {
        // Isolate activation requests from ordinary new-window focus decisions.
        let config = format!(
            "debug {{ strict-new-window-focus-policy; }}\n{}",
            activation_config(policy),
        );
        for (with_serial, expected) in [(true, valid), (false, serialless)] {
            assert_eq!(
                target_state_after_activation_before_mapping(&config, with_serial),
                expected,
                "policy {policy:?}, with_serial {with_serial}",
            );
        }
    }
}

#[test]
fn open_focused_overrides_activation_focus_before_mapping() {
    assert_eq!(
        target_state_after_activation_before_mapping(
            r#"
            window-rule {
                open-focused false
                on-xdg-activate "focus"
            }
            "#,
            false,
        ),
        (false, false),
    );
}

#[test]
fn serialless_activation_before_mapping_is_urgent_with_open_focused_false() {
    assert_eq!(
        target_state_after_activation_before_mapping(
            r#"
            window-rule {
                open-focused false
            }
            "#,
            false,
        ),
        (false, true),
    );
}

#[test]
fn ignored_activation_window_remains_directly_focusable() {
    let (mut fixture, target) = mapped_target_with_focused_peer(&activation_config(Some("ignore")));

    fixture.niri().layout.activate_window(&target.window);

    let focused_window = fixture
        .niri()
        .layout
        .focus()
        .map(|mapped| mapped.window.clone());
    assert_eq!(focused_window, Some(target.window));
}

fn activation_config(policy: Option<&str>) -> String {
    policy
        .map(|policy| format!("window-rule {{ on-xdg-activate \"{policy}\"; }}"))
        .unwrap_or_default()
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

fn target_state_after_activation_before_mapping(config: &str, with_serial: bool) -> (bool, bool) {
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

    // Strict new-window focus leaves the source unfocused; establish actual input focus
    // so valid-serial cases exercise a token supported by a keyboard enter event.
    fixture.niri().layout.activate_window(&source_window);
    fixture.double_roundtrip(client_id);

    let target = fixture.client(client_id).create_window();
    let target_surface = target.surface.clone();
    target.commit();
    fixture.roundtrip(client_id);

    let serial = with_serial.then(|| fixture.client(client_id).keyboard_enter_serial());
    let token_result = fixture.client(client_id).request_activation_token(serial);
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
