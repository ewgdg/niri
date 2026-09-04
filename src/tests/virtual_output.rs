use std::sync::atomic::Ordering;
use std::time::Duration;

use super::*;

#[test]
fn visible_surface_on_virtual_output_receives_presentation_feedback() {
    let mut f = Fixture::new();
    {
        let state = f.niri_state();
        state
            .backend
            .create_virtual_output(&mut state.niri, 1920, 1080, 60, Some("virt".to_owned()))
            .unwrap();
    }

    let id = f.add_client();
    let window = f.client(id).create_window();
    let surface = window.surface.clone();
    window.commit();
    f.roundtrip(id);

    let window = f.client(id).window(&surface);
    window.attach_new_buffer();
    window.ack_last_and_commit();
    f.double_roundtrip(id);

    let feedback = f.client(id).request_presentation_feedback(&surface);
    f.client(id).window(&surface).commit();
    f.roundtrip(id);

    // Presentation feedback is emitted on the virtual output's next 60 Hz refresh.
    std::thread::sleep(Duration::from_millis(20));
    f.double_roundtrip(id);

    let feedback = feedback.data.lock().unwrap();
    assert!(feedback.presented);
    assert!(!feedback.discarded);
}

#[test]
fn virtual_output_frame_callbacks_are_paced_to_refresh() {
    let mut fixture = Fixture::new();
    {
        let state = fixture.niri_state();
        state
            .backend
            .create_virtual_output(&mut state.niri, 1920, 1080, 2, Some("virt".to_owned()))
            .unwrap();
    }

    let client_id = fixture.add_client();
    let window = fixture.client(client_id).create_window();
    let surface = window.surface.clone();
    window.commit();
    fixture.roundtrip(client_id);

    // Let the initial output redraw finish so the first client commit starts a fresh refresh cycle.
    std::thread::sleep(Duration::from_millis(550));
    fixture.dispatch();

    let first_callback = {
        let window = fixture.client(client_id).window(&surface);
        window.attach_new_buffer();
        window.ack_last();
        let callback = window.request_frame_callback();
        window.commit();
        callback
    };
    fixture.double_roundtrip(client_id);
    assert!(first_callback.done.load(Ordering::Relaxed));

    let second_callback = {
        let window = fixture.client(client_id).window(&surface);
        let callback = window.request_frame_callback();
        window.commit();
        callback
    };
    fixture.double_roundtrip(client_id);
    assert!(!second_callback.done.load(Ordering::Relaxed));

    std::thread::sleep(Duration::from_millis(550));
    fixture.double_roundtrip(client_id);
    assert!(second_callback.done.load(Ordering::Relaxed));
}

#[test]
fn virtual_output_custom_mode_does_not_accumulate_modes() {
    let mut f = Fixture::new();

    // Create a managed virtual output so it goes through the same config application path as in a
    // real session (`niri msg create-virtual-output`, `niri msg output ... custom-mode`).
    let name = {
        let state = f.niri_state();
        state
            .backend
            .create_virtual_output(&mut state.niri, 1920, 1080, 60, Some("sunshine".to_owned()))
            .unwrap()
    };

    let output = f
        .niri()
        .global_space
        .outputs()
        .find(|o| o.name() == name)
        .unwrap()
        .clone();

    // Sanity: single initial mode.
    {
        let modes = output.modes();
        assert_eq!(modes.len(), 1);
        assert_eq!(modes[0].size.w, 1920);
        assert_eq!(modes[0].size.h, 1080);
    }

    // 1080p -> 3200x1800
    {
        let state = f.niri_state();
        state.apply_transient_output_config(
            &name,
            niri_ipc::OutputAction::CustomMode {
                mode: niri_ipc::ConfiguredMode {
                    width: 3200,
                    height: 1800,
                    refresh: Some(60.0),
                },
            },
        );
    }

    {
        let modes = output.modes();
        assert_eq!(modes.len(), 1);
        assert_eq!(modes[0].size.w, 3200);
        assert_eq!(modes[0].size.h, 1800);
    }

    // 3200x1800 -> 1080p
    {
        let state = f.niri_state();
        state.apply_transient_output_config(
            &name,
            niri_ipc::OutputAction::CustomMode {
                mode: niri_ipc::ConfiguredMode {
                    width: 1920,
                    height: 1080,
                    refresh: Some(60.0),
                },
            },
        );
    }

    {
        let modes = output.modes();
        assert_eq!(modes.len(), 1);
        assert_eq!(modes[0].size.w, 1920);
        assert_eq!(modes[0].size.h, 1080);
    }
}

#[test]
fn off_virtual_output_can_be_removed() {
    let mut f = Fixture::new();

    let name = {
        let state = f.niri_state();
        state
            .backend
            .create_virtual_output(&mut state.niri, 1920, 1080, 60, Some("virt".to_owned()))
            .unwrap()
    };

    f.niri_state()
        .apply_transient_output_config(&name, niri_ipc::OutputAction::Off);

    {
        let state = f.niri_state();
        state
            .backend
            .remove_virtual_output(&mut state.niri, &name)
            .unwrap();
    }

    let state = f.niri_state();
    assert!(state
        .backend
        .remove_virtual_output(&mut state.niri, &name)
        .is_err());
}

#[test]
fn configured_virtual_output_cannot_be_removed_at_runtime() {
    let mut f = Fixture::new();
    let name = "virt";

    {
        let state = f.niri_state();
        state.modify_output_config(name, |config| config.create_virtual = true);
        state.reload_output_config();
    }

    let result = {
        let state = f.niri_state();
        state.backend.remove_virtual_output(&mut state.niri, name)
    };

    assert!(result.is_err_and(|err| err.contains("is configured")));
    assert!(f
        .niri_state()
        .backend
        .ipc_outputs()
        .lock()
        .unwrap()
        .values()
        .any(|output| output.name == name));
}

#[test]
fn virtual_output_name_is_reported_as_its_model() {
    let mut f = Fixture::new();
    let name = {
        let state = f.niri_state();
        state
            .backend
            .create_virtual_output(&mut state.niri, 1920, 1080, 60, Some("sunshine".to_owned()))
            .unwrap()
    };

    let ipc_outputs = f.niri_state().backend.ipc_outputs();
    let ipc_outputs = ipc_outputs.lock().unwrap();
    let output = ipc_outputs
        .values()
        .find(|output| output.name == name)
        .unwrap();

    assert_eq!(output.model, name);
}

#[test]
fn touch_input_targets_virtual_output_when_focused() {
    let mut f = Fixture::new();
    f.add_output(1, (1920, 1080));

    // Create a virtual output and focus it.
    let name = {
        let state = f.niri_state();
        state
            .backend
            .create_virtual_output(&mut state.niri, 1920, 1080, 60, Some("virt".to_owned()))
            .unwrap()
    };

    let virt = f
        .niri()
        .global_space
        .outputs()
        .find(|o| o.name() == name)
        .unwrap()
        .clone();

    f.niri().layout.focus_output(&virt);

    // With no explicit `input.touch.map-to-output` configured, touch should follow the active
    // output (which may be virtual).
    let touch_output = f.niri().output_for_touch().unwrap().clone();
    assert_eq!(touch_output, virt);
}
