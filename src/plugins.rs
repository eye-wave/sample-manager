use std::{
    path::PathBuf,
    sync::{Arc, mpsc},
};

use plugin_wire::{WireEntry, sample::SampleSerialize};
use wasmtime::{Instance, TypedFunc};

use crate::{
    AStr, AnyResult,
    ipc::IPCMessage,
    plugins::{manifest::PluginManifest, runner::PluginRunner},
    state::samples::SearchRequest,
};

mod host;
mod loader;
mod manifest;
mod runner;

pub(super) use loader::unpack_plugin_zip;

pub use manifest::{PluginId, PluginInfo, config_key, parse_string_to_bytes};

pub struct PluginInstance {
    pub instance: Instance,
    pub manifest: PluginManifest,
    pub fn_search: TypedFunc<(u32, u32), u32>,
    pub fn_get_index: Option<TypedFunc<(u32, u32), u32>>,
    pub fn_alloc: TypedFunc<u32, u32>,
    pub fn_free: TypedFunc<(u32, u32), ()>,
}

#[derive(Debug)]
pub enum PluginRunnerCommand {
    LoadPlugin {
        name: AStr,
        bytes: Vec<u8>,
    },
    Search {
        id: PluginId,
        req: SearchRequest,
        reply_to: mpsc::SyncSender<Result<Vec<SampleSerialize>, String>>,
    },
    SetConfigField {
        id: PluginId,
        name: AStr,
        data: Vec<u8>,
    },
    UnloadPlugin {
        id: PluginId,
    },
    GetAllPluginsInfo {
        reply_to: mpsc::Sender<Vec<PluginInfo>>,
    },
    Download {
        plugin_id: PluginId,
        url: String,
        reply_to: mpsc::SyncSender<Result<PathBuf, String>>,
        ffmpeg_path: Option<AStr>,
        web_sender: mpsc::Sender<IPCMessage>,
    },
    SearchLocalRegistry {
        req: SearchRequest,
        reply_to: mpsc::SyncSender<Arc<Vec<WireEntry>>>,
    },
}

use PluginRunnerCommand as Cmd;

pub struct PluginRuntimeHandle {
    sender: mpsc::Sender<PluginRunnerCommand>,
}

impl PluginRuntimeHandle {
    pub fn spawn() -> Self {
        let (tx, rx) = mpsc::channel::<Cmd>();

        std::thread::Builder::new()
            .name("plugin-runner".into())
            .spawn(move || match PluginRunner::new() {
                Ok(plug) => plug.run(rx),
                Err(err) => {
                    eprintln!("{err}");
                }
            })
            .expect("failed to spawn plugin runner thread");

        Self { sender: tx }
    }

    pub fn load(&self, name: &str, bytes: Vec<u8>) {
        let _ = self.sender.send(Cmd::LoadPlugin {
            bytes,
            name: Arc::from(name),
        });
    }

    pub fn set_config_field(&self, id: PluginId, name: AStr, data: Vec<u8>) {
        let _ = self.sender.send(Cmd::SetConfigField {
            id,
            name: name.clone(),
            data,
        });
    }

    pub fn search_local_registry(&self, req: &SearchRequest) -> AnyResult<Arc<Vec<WireEntry>>> {
        let (tx, rx) = mpsc::sync_channel(1);

        self.sender.send(Cmd::SearchLocalRegistry {
            req: req.clone(),
            reply_to: tx,
        })?;

        Ok(rx.recv()?)
    }

    pub fn download(
        &self,
        plugin_id: PluginId,
        url: &str,
        ffmpeg_path: Option<AStr>,
        web_sender: mpsc::Sender<IPCMessage>,
    ) -> Result<PathBuf, String> {
        let (tx, rx) = mpsc::sync_channel(1);

        self.sender
            .send(Cmd::Download {
                plugin_id,
                url: url.to_string(),
                reply_to: tx,
                ffmpeg_path,
                web_sender,
            })
            .map_err(|e| e.to_string())?;

        rx.recv().map_err(|e| e.to_string())?
    }

    pub fn search(&self, id: PluginId, req: SearchRequest) -> Result<Vec<SampleSerialize>, String> {
        let (tx, rx) = mpsc::sync_channel(1);
        self.sender
            .send(Cmd::Search {
                id,
                req,
                reply_to: tx,
            })
            .map_err(|e| e.to_string())?;

        rx.recv().map_err(|e| e.to_string())?
    }

    pub fn get_all_plugins_info(&self) -> Vec<PluginInfo> {
        let (tx, rx) = std::sync::mpsc::channel();
        let _ = self.sender.send(Cmd::GetAllPluginsInfo { reply_to: tx });
        rx.recv().unwrap_or_default()
    }

    pub fn unload(&self, id: PluginId) {
        let _ = self.sender.send(Cmd::UnloadPlugin { id });
    }
}
