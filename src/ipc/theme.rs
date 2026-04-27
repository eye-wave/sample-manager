use std::fs;

use crate::{
    ipc::{IPCBody, IPCError, IPCResponse, IntoIPCResponse},
    ipc_commands,
    state::{
        AppDirs,
        config::{Theme, ThemeType},
    },
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
    let mut files = fs::read_dir(AppDirs::themes_path())?
        .filter_map(Result::ok)
        .filter_map(|f| {
            let path = AppDirs::themes_path().join(f.path());
            let theme: Theme = toml::from_str(&fs::read_to_string(&path).ok()?).ok()?;

            Some((
                theme.theme_type,
                f.file_name().to_string_lossy().into_owned(),
            ))
        })
        .collect::<Vec<_>>();

    let light_count = files.iter().filter(|e| e.0 == ThemeType::Light).count();

    files.sort_by_key(|a| a.0);

    format!(
        "{light_count},{}",
        files
            .iter()
            .map(|f| f.1.clone())
            .intersperse(",".into())
            .collect::<String>()
    )
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
