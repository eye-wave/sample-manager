use std::path::Path;

use crate::audio::PlaybackState;
use crate::ipc::{IPCBody, IPCResponse, IntoIPCResponse, Poisoned, ok};

fn play_audio_file(body: IPCBody) -> IPCResponse {
    crate::with_state!(body, state, {
        let path = Path::new(&*body.req);

        state.audio_player.play(&path)?;
        ok()
    })
}

fn player_pause(body: IPCBody) -> IPCResponse {
    crate::with_state!(body, state, {
        state.audio_player.pause()?;
        ok()
    })
}

fn player_resume(body: IPCBody) -> IPCResponse {
    crate::with_state!(body, state, {
        state.audio_player.resume()?;
        ok()
    })
}

fn player_stop(body: IPCBody) -> IPCResponse {
    crate::with_state!(body, state, {
        state.audio_player.stop()?;
        ok()
    })
}

fn get_playback_state(body: IPCBody) -> IPCResponse {
    crate::with_state!(body, state, {
        match state.audio_player.playback_state() {
            PlaybackState::Paused => 0,
            PlaybackState::Playing => 1,
            PlaybackState::Stopped => 2,
        }
        .to_string()
        .finish()
    })
}

fn get_audio_position(body: IPCBody) -> IPCResponse {
    crate::with_state!(body, state, {
        state.audio_player.position().to_string().finish()
    })
}

fn player_seek(body: IPCBody) -> IPCResponse {
    crate::with_state!(body, state, {
        let millis = body.req.parse()?;
        state.audio_player.seek(millis)?;

        ok()
    })
}

crate::ipc_commands! {
    IPC_AUDIO = [
        get_audio_position,
        get_playback_state,
        play_audio_file,
        player_pause,
        player_resume,
        player_stop,
        player_seek
    ]
}
