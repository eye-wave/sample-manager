use std::path::Path;

use crate::audio::PlaybackState;
use crate::ipc::{IPCBody, IPCResponse, IntoIPCResponse, Poisoned, ok};

macro_rules! ipc_action {
    ($name:ident, $state:ident => $body:block) => {
        fn $name(body: IPCBody) -> IPCResponse {
            crate::with_state!(body, $state, {
                $body
                ok()
            })
        }
    };
}

ipc_action!(player_pause, state => {
    state.audio_player.pause()?;
});

ipc_action!(player_resume, state => {
    state.audio_player.resume()?;
});

ipc_action!(player_stop, state => {
    state.audio_player.stop()?;
});

fn play_audio_file(body: IPCBody) -> IPCResponse {
    crate::with_state!(body, state, {
        let path = Path::new(&*body.req);

        state.audio_player.play(&path)?;
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

fn get_volume(body: IPCBody) -> IPCResponse {
    crate::with_state!(body, state, {
        state.audio_player.get_volume().to_string().finish()
    })
}

fn set_volume(body: IPCBody) -> IPCResponse {
    crate::with_state!(body, state, {
        let volume = body.req.parse()?;
        state.audio_player.set_volume(volume);

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
        player_seek,
        get_volume,
        set_volume,
    ]
}
