use std::fs;

use crate::{
    ipc::{IPCBody, IPCError, IPCResponse, IntoIPCResponse},
    ipc_commands,
    state::AppDirs,
};

fn get_theme(body: IPCBody) -> IPCResponse {
    let guard = body.app_state.read().unwrap();
    let theme = guard
        .get_config()
        .get_current_theme()
        .unwrap_or_default()
        .to_css();

    theme.finish()
}

fn preview_theme(body: IPCBody) -> IPCResponse {
    let theme_name = body.req;
    let guard = body.app_state.write().unwrap();

    guard
        .get_config()
        .get_theme(&theme_name)
        .ok_or(IPCError::empty())?
        .to_css()
        .finish()
}

fn update_theme(body: IPCBody) -> IPCResponse {
    let theme_name = body.req;
    let mut guard = body.app_state.write().unwrap();

    let theme = guard.update_config(|cfg| cfg.update_theme(&theme_name));
    theme.ok_or(IPCError::empty())?.to_css().finish()
}

fn get_theme_name(body: IPCBody) -> IPCResponse {
    let guard = body.app_state.read().unwrap();
    let theme = guard
        .get_config()
        .color_theme
        .as_ref()
        .ok_or(IPCError::empty())?;

    theme.clone().finish()
}

fn list_themes(_: IPCBody) -> IPCResponse {
    fs::read_dir(AppDirs::themes_path())?
        .filter_map(Result::ok)
        .filter(|f| f.path().extension().is_some_and(|ext| ext == "toml"))
        .map(|f| f.file_name().to_string_lossy().into_owned())
        .intersperse(",".into())
        .collect::<String>()
        .finish()
}

ipc_commands! {
    IPC_THEME = [
        get_theme,
        preview_theme,
        update_theme,
        get_theme_name,
        list_themes
    ]
}
