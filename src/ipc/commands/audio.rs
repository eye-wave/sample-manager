use std::path::Path;

use crate::{
    audio::PlaybackState,
    ipc::{IPCBody, IPCResponse, IntoIPCResponse, ok},
};

fn player_pause(body: IPCBody) -> IPCResponse {
    let state = body.read_state()?;
    state.audio_player.pause()?;
    ok()
}

fn player_resume(body: IPCBody) -> IPCResponse {
    let state = body.read_state()?;
    state.audio_player.resume()?;
    ok()
}

fn player_stop(body: IPCBody) -> IPCResponse {
    let state = body.read_state()?;
    state.audio_player.stop()?;
    ok()
}

fn play_audio_file(body: IPCBody) -> IPCResponse {
    let state = body.read_state()?;
    let path = Path::new(&*body.req);

    state.audio_player.play(&path).map(|_| ok())?
}

fn get_playback_state(body: IPCBody) -> IPCResponse {
    let state = body.read_state()?;

    match state.audio_player.playback_state() {
        PlaybackState::Paused => 0,
        PlaybackState::Playing => 1,
        PlaybackState::Stopped => 2,
    }
    .to_string()
    .finish()
}

fn get_audio_position(body: IPCBody) -> IPCResponse {
    let state = body.read_state()?;
    state.audio_player.position().to_string().finish()
}

fn get_audio_position_pretty(body: IPCBody) -> IPCResponse {
    let state = body.read_state()?;
    state.audio_player.position_pretty().finish()
}

fn player_seek(body: IPCBody) -> IPCResponse {
    let state = body.read_state()?;
    let millis = body.parse_req()?;

    state.audio_player.seek(millis)?;

    ok()
}

fn get_volume(body: IPCBody) -> IPCResponse {
    let state = body.read_state()?;
    state.audio_player.get_volume().to_string().finish()
}

fn set_volume(body: IPCBody) -> IPCResponse {
    let state = body.read_state()?;

    let volume = body.req.parse()?;
    state.audio_player.set_volume(volume);

    ok()
}

fn get_looping(body: IPCBody) -> IPCResponse {
    let state = body.read_state()?;
    state.audio_player.is_looping().finish()
}

fn set_looping(body: IPCBody) -> IPCResponse {
    let state = body.read_state()?;
    let looping = body.req.chars().next().map(|c| c == '1').unwrap_or(false);
    state.audio_player.set_looping(looping);

    ok()
}

crate::ipc_commands! {
    IPC_AUDIO = [
        get_audio_position_pretty,
        get_audio_position,
        get_playback_state,
        play_audio_file,
        player_resume,
        player_pause,
        player_stop,
        player_seek,
        get_looping,
        set_looping,
        get_volume,
        set_volume
    ]
}
