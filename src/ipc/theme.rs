use crate::ipc::{IPCBody, IPCError, IPCResponse, IntoIPCResponse};
use crate::ipc_commands;

fn get_theme(body: IPCBody) -> IPCResponse {
    crate::with_state!(body, state, {
        let theme = state
            .get_config()
            .get_current_theme()
            .unwrap_or_default()
            .to_css();

        theme.finish()
    })
}

fn preview_theme(body: IPCBody) -> IPCResponse {
    let theme_name = body.req;

    crate::with_state!(body, state, {
        state
            .get_config()
            .get_theme(&theme_name)
            .ok_or(IPCError::empty())?
            .to_css()
            .finish()
    })
}

fn update_theme(body: IPCBody) -> IPCResponse {
    let theme_name = body.req;

    crate::with_state_mut!(body, state, {
        let theme = state.update_config(|cfg| cfg.update_theme(&theme_name));
        theme.ok_or(IPCError::empty())?.to_css().finish()
    })
}

fn get_theme_name(body: IPCBody) -> IPCResponse {
    crate::with_state!(body, state, {
        let theme = state
            .get_config()
            .color_theme
            .as_ref()
            .ok_or(IPCError::empty())?;

        theme.clone().finish()
    })
}

ipc_commands! {
    IPC_THEME = [
        get_theme,
        preview_theme,
        update_theme,
        get_theme_name,
    ]
}
