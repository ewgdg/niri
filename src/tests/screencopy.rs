use std::sync::{Arc, Mutex};

use smithay::reexports::wayland_protocols_wlr::screencopy::v1::client::zwlr_screencopy_frame_v1::ZwlrScreencopyFrameV1;

use super::client::{ClientId, ScreencopyBufferParams, ScreencopyFrameData, ShmBuffer};
use super::Fixture;

fn set_up() -> (Fixture, ClientId) {
    let mut f = Fixture::new();
    f.niri_state().backend.headless().add_renderer().unwrap();
    f.add_output(1, (64, 48));
    let id = f.add_client();
    f.roundtrip(id);
    (f, id)
}

fn wait_for(
    f: &mut Fixture,
    data: &Arc<Mutex<ScreencopyFrameData>>,
    desc: &str,
    pred: impl Fn(&ScreencopyFrameData) -> bool,
) {
    for _ in 0..200 {
        f.dispatch();
        if pred(&data.lock().unwrap()) {
            return;
        }
    }

    panic!("timed out waiting for {desc}");
}

fn capture_params_for_output(
    f: &mut Fixture,
    id: ClientId,
    output_name: &str,
) -> (
    ZwlrScreencopyFrameV1,
    Arc<Mutex<ScreencopyFrameData>>,
    ScreencopyBufferParams,
) {
    let output = f.client(id).output(output_name);
    let frame = f.client(id).capture_output(&output);
    wait_for(f, &frame.data, "screencopy buffer params", |data| {
        data.buffer_done && data.buffer.is_some()
    });
    let params = frame.data.lock().unwrap().buffer.unwrap();
    (frame.frame, frame.data, params)
}

fn capture_params(
    f: &mut Fixture,
    id: ClientId,
) -> (
    ZwlrScreencopyFrameV1,
    Arc<Mutex<ScreencopyFrameData>>,
    ScreencopyBufferParams,
) {
    capture_params_for_output(f, id, "headless-1")
}

fn copy_with_damage(
    f: &mut Fixture,
    id: ClientId,
    frame: &ZwlrScreencopyFrameV1,
    params: ScreencopyBufferParams,
) -> ShmBuffer {
    let buffer = f.client(id).create_shm_buffer(params);
    frame.copy_with_damage(&buffer.buffer);
    f.client(id).connection.flush().unwrap();
    f.dispatch();
    buffer
}

#[test]
fn copy_with_damage_queue_replaces_older_same_output() {
    let (mut f, id) = set_up();

    let (old_frame, old_data, old_params) = capture_params(&mut f, id);
    let _old_buffer = copy_with_damage(&mut f, id, &old_frame, old_params);

    let (new_frame, new_data, new_params) = capture_params(&mut f, id);
    let new_buffer = copy_with_damage(&mut f, id, &new_frame, new_params);

    wait_for(&mut f, &old_data, "older frame failure", |data| data.failed);
    {
        let new_data = new_data.lock().unwrap();
        assert!(!new_data.failed, "newest frame must remain queued");
        assert!(
            !new_data.ready,
            "newest frame must not complete before power-off drain"
        );
    }

    let state = f.niri_state();
    state.niri.deactivate_monitors(&mut state.backend);
    state.refresh_and_flush_clients();
    wait_for(&mut f, &new_data, "newest frame black ready", |data| {
        data.ready
    });

    let new_data = new_data.lock().unwrap();
    assert!(!new_data.failed);
    assert_eq!(
        new_data.damages,
        [(0, 0, new_params.width, new_params.height)]
    );
    assert!(new_buffer.pixels().iter().all(|pixel| *pixel == 0xFF000000));
}

#[test]
fn removed_output_still_fails_queued_screencopy() {
    let (mut f, id) = set_up();

    let (frame, data, params) = capture_params(&mut f, id);
    let _buffer = copy_with_damage(&mut f, id, &frame, params);

    let output = f.niri_output(1);
    f.niri().remove_output(&output);
    f.niri_state().refresh_and_flush_clients();

    wait_for(
        &mut f,
        &data,
        "queued frame failure after output removal",
        |data| data.failed,
    );
    let data = data.lock().unwrap();
    assert!(!data.ready);
}

#[test]
fn managed_virtual_output_off_black_submits_queued_screencopy() {
    let mut f = Fixture::new();
    f.niri_state().backend.headless().add_renderer().unwrap();
    let name = {
        let state = f.niri_state();
        state
            .backend
            .create_virtual_output(&mut state.niri, 64, 48, 60, Some("sunshine".to_owned()))
            .unwrap()
    };
    let id = f.add_client();
    f.roundtrip(id);

    let (frame, data, params) = capture_params_for_output(&mut f, id, &name);
    let buffer = copy_with_damage(&mut f, id, &frame, params);

    f.niri_state()
        .apply_transient_output_config(&name, niri_ipc::OutputAction::Off);
    f.niri_state().refresh_and_flush_clients();

    wait_for(&mut f, &data, "virtual-output off black ready", |data| {
        data.ready
    });
    let data = data.lock().unwrap();
    assert!(!data.failed);
    assert_eq!(data.damages, [(0, 0, params.width, params.height)]);
    assert!(buffer.pixels().iter().all(|pixel| *pixel == 0xFF000000));
    assert!(f.niri().global_space.outputs().all(|output| output.name() != name));
}
