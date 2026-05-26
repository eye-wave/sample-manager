use std::path::Path;

use crate::audio::PlaybackState;
use crate::ipc::{IPCBody, IPCResponse, IntoIPCResponse, ok};

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

        state.audio_player.play(&path).map(|_| ok())?
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

fn get_audio_position_pretty(body: IPCBody) -> IPCResponse {
    crate::with_state!(body, state, {
        state.audio_player.position_pretty().finish()
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

fn get_looping(body: IPCBody) -> IPCResponse {
    crate::with_state!(body, state, {
        (state.audio_player.is_looping() as u8).to_string().finish()
    })
}

fn set_looping(body: IPCBody) -> IPCResponse {
    crate::with_state!(body, state, {
        let looping = body.req.chars().next().map(|c| c == '1').unwrap_or(false);
        state.audio_player.set_looping(looping);

        ok()
    })
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
