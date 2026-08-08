use niri_config::Config;
use wayland_client::Proxy as _;
use wayland_server::Resource as _;

use super::*;
use crate::tests::client::ClientId;

fn hidden_config() -> Config {
    Config::parse_mem(
        r#"
        window-rule {
            match app-id="^hidden-app$"
            hidden true
        }
        "#,
    )
    .unwrap()
}

/// Maps a window with app-id "hidden-app" and returns the server-side surface id of the hidden
/// window, or panics if there is no hidden window.
fn map_hidden_window(f: &mut Fixture, id: ClientId) -> u32 {
    let window = f.client(id).create_window();
    let surface = window.surface.clone();
    window.set_app_id("hidden-app");
    window.commit();
    f.roundtrip(id);

    let window = f.client(id).window(&surface);
    window.attach_new_buffer();
    window.ack_last_and_commit();
    f.double_roundtrip(id);

    let hidden = f
        .niri()
        .hidden_windows
        .values()
        .next()
        .unwrap_or_else(|| panic!("window must be hidden"))
        .window
        .toplevel()
        .unwrap()
        .wl_surface()
        .id()
        .protocol_id();
    assert_eq!(hidden, surface.id().protocol_id());
    hidden
}

#[test]
fn hidden_window_is_not_added_to_layout() {
    let mut f = Fixture::with_config(hidden_config());
    f.add_output(1, (1920, 1080));

    let id = f.add_client();
    let _ = map_hidden_window(&mut f, id);

    // The window must not be in the layout, so it is not rendered, focused, or otherwise
    // visible to the user.
    assert!(f.niri().layout.windows().next().is_none());
    assert_eq!(f.niri().hidden_windows.len(), 1);
    assert!(f.niri().layout.focus().is_none());
}

#[test]
fn hidden_window_unmap_remap_stays_hidden() {
    let mut f = Fixture::with_config(hidden_config());
    f.add_output(1, (1920, 1080));

    let id = f.add_client();
    let surface = f.client(id).create_window();
    let client_surface = surface.surface.clone();
    surface.set_app_id("hidden-app");
    surface.commit();
    f.roundtrip(id);

    let surface = f.client(id).window(&client_surface);
    surface.attach_new_buffer();
    surface.ack_last_and_commit();
    f.double_roundtrip(id);

    assert_eq!(f.niri().hidden_windows.len(), 1);

    // Unmap the hidden window; it goes back to the normal unmapped flow.
    let surface = f.client(id).window(&client_surface);
    surface.attach_null();
    surface.commit();
    f.double_roundtrip(id);

    assert!(f.niri().hidden_windows.is_empty());
    assert_eq!(f.niri().unmapped_windows.len(), 1);

    // Remap it: the client must redo the initial configure → map sequence (re-sending its
    // properties, since they are discarded on unmap), after which the window gets hidden again.
    let surface = f.client(id).window(&client_surface);
    surface.set_app_id("hidden-app");
    surface.commit();
    f.roundtrip(id);

    let surface = f.client(id).window(&client_surface);
    surface.attach_new_buffer();
    surface.ack_last_and_commit();
    f.double_roundtrip(id);

    assert_eq!(f.niri().hidden_windows.len(), 1);
    assert!(f.niri().unmapped_windows.is_empty());
    assert!(f.niri().layout.windows().next().is_none());
}

#[test]
fn config_reload_hides_mapped_window() {
    let mut f = Fixture::new();
    f.add_output(1, (1920, 1080));

    let id = f.add_client();
    let window = f.client(id).create_window();
    let surface = window.surface.clone();
    window.set_app_id("hidden-app");
    window.commit();
    f.roundtrip(id);

    let window = f.client(id).window(&surface);
    window.attach_new_buffer();
    window.ack_last_and_commit();
    f.double_roundtrip(id);

    assert!(f.niri().layout.windows().next().is_some());

    // Adding a matching hidden rule must hide the already-mapped window on reload.
    f.niri().config.borrow_mut().window_rules = hidden_config().window_rules;
    f.niri().recompute_window_rules();

    assert!(f.niri().layout.windows().next().is_none());
    assert_eq!(f.niri().hidden_windows.len(), 1);
}

#[test]
fn config_reload_restores_placement() {
    let mut f = Fixture::new();
    f.add_output(1, (1920, 1080));

    let id = f.add_client();
    let window = f.client(id).create_window();
    let surface = window.surface.clone();
    window.set_app_id("hidden-app");
    window.commit();
    f.roundtrip(id);

    let window = f.client(id).window(&surface);
    window.attach_new_buffer();
    window.ack_last_and_commit();
    f.double_roundtrip(id);

    let (_, mapped) = f.niri().layout.windows().next().unwrap();
    let size_before = mapped.window.geometry().size;
    let window = mapped.window.clone();
    let workspace_id_before = f
        .niri()
        .layout
        .workspaces()
        .find_map(|(_, _, ws)| ws.has_window(&window).then(|| ws.id()));

    // Hide the window with a reload, then unhide it with another reload.
    f.niri().config.borrow_mut().window_rules = hidden_config().window_rules;
    f.niri().recompute_window_rules();
    assert_eq!(f.niri().hidden_windows.len(), 1);

    f.niri().config.borrow_mut().window_rules = Vec::new();
    f.niri().recompute_window_rules();

    assert!(f.niri().hidden_windows.is_empty());
    let (_, mapped) = f.niri().layout.windows().next().unwrap();
    assert_eq!(mapped.window.geometry().size, size_before);
    let window = mapped.window.clone();
    let workspace_id_after = f
        .niri()
        .layout
        .workspaces()
        .find_map(|(_, _, ws)| ws.has_window(&window).then(|| ws.id()));
    assert_eq!(workspace_id_after, workspace_id_before);
}

#[test]
fn config_reload_unhides_window() {
    let mut f = Fixture::with_config(hidden_config());
    f.add_output(1, (1920, 1080));

    let id = f.add_client();
    let _ = map_hidden_window(&mut f, id);
    assert_eq!(f.niri().hidden_windows.len(), 1);

    // Removing the hidden rule must reveal the window on reload.
    f.niri().config.borrow_mut().window_rules = Vec::new();
    f.niri().recompute_window_rules();

    assert!(f.niri().hidden_windows.is_empty());
    assert!(f.niri().layout.windows().next().is_some());
}
