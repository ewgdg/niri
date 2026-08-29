use std::sync::atomic::Ordering;
use std::time::Duration;

use super::*;

#[test]
fn fallback_frame_callbacks_pause_while_monitors_are_inactive() {
    let mut fixture = Fixture::new();
    fixture.add_output(1, (1920, 1080));

    let client_id = fixture.add_client();
    let window = fixture.client(client_id).create_window();
    let surface = window.surface.clone();
    window.commit();
    fixture.roundtrip(client_id);

    {
        let state = fixture.niri_state();
        state.niri.deactivate_monitors(&mut state.backend);
    }

    let callback = {
        let window = fixture.client(client_id).window(&surface);
        window.attach_new_buffer();
        window.ack_last();
        let callback = window.request_frame_callback();
        window.commit();
        callback
    };
    fixture.roundtrip(client_id);

    fixture.niri().send_frame_callbacks_on_fallback_timer();
    fixture.roundtrip(client_id);
    assert!(!callback.done.load(Ordering::Relaxed));

    {
        let state = fixture.niri_state();
        state.niri.activate_monitors(&mut state.backend);
        state.refresh_and_flush_clients();
    }

    // Reactivation resumes callbacks on the next output refresh.
    std::thread::sleep(Duration::from_millis(20));
    fixture.roundtrip(client_id);
    assert!(callback.done.load(Ordering::Relaxed));
}
