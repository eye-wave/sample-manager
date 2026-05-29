use std::path::PathBuf;
use std::sync::Arc;
use std::sync::mpsc::{self};

use plugin_wire::{WireEntry, sample::SampleSerialize};
use wasmtime::{Instance, TypedFunc};

use crate::ipc::IPCSenderUI;
use crate::state::config::FFPaths;
use crate::state::samples::SearchRequest;
use crate::{AStr, LogErrorExt};

mod host;
mod icon;
mod manifest;
mod runner;

use icon::SVGIcon;
use manifest::PluginManifest;
use runner::PluginRunner;

pub use manifest::{PluginId, PluginInfo};

pub struct PluginInstance {
    pub instance: Instance,
    pub manifest: PluginManifest,
    pub fn_search: TypedFunc<(u32, u32), u32>,
    pub fn_get_index: Option<TypedFunc<(u32, u32), u32>>,
    pub fn_alloc: TypedFunc<u32, u32>,
    pub fn_free: TypedFunc<(u32, u32), ()>,
}

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
    GetPluginInfo {
        id: PluginId,
        reply_to: mpsc::SyncSender<Option<PluginInfo>>,
    },
    GetAllPluginsInfo {
        reply_to: mpsc::Sender<Vec<PluginInfo>>,
        icon_cb: Box<dyn Fn(&mut SVGIcon) + Send + 'static>,
    },
    Download {
        plugin_id: PluginId,
        url: String,
        reply_to: mpsc::SyncSender<Result<PathBuf, String>>,
        ffpaths: FFPaths,
        web_sender: IPCSenderUI,
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

#[derive(Debug, thiserror::Error)]
pub enum PluginSendError {
    #[error("MPSC send error")]
    Send(#[from] mpsc::SendError<PluginRunnerCommand>),

    #[error("MPSC receive error")]
    Recv(#[from] mpsc::RecvError),
}

impl PluginRuntimeHandle {
    pub fn spawn() -> Self {
        let (tx, rx) = mpsc::channel::<Cmd>();

        std::thread::Builder::new()
            .name("plugin-runner".into())
            .spawn(move || match PluginRunner::new() {
                Ok(plug) => plug.run(rx),
                Err(err) => {
                    tracing::error!(error = %err);
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

    pub fn search_local_registry(
        &self,
        req: &SearchRequest,
    ) -> Result<Arc<Vec<WireEntry>>, PluginSendError> {
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
        ffpaths: FFPaths,
        web_sender: IPCSenderUI,
    ) -> Result<PathBuf, String> {
        let (tx, rx) = mpsc::sync_channel(1);

        self.sender
            .send(Cmd::Download {
                plugin_id,
                url: url.to_string(),
                reply_to: tx,
                ffpaths,
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

    pub fn get_all_plugins_info<F>(&self, icon_cb: F) -> Vec<PluginInfo>
    where
        F: Fn(&mut SVGIcon) + Send + 'static,
    {
        let (tx, rx) = std::sync::mpsc::channel();
        let _ = self.sender.send(Cmd::GetAllPluginsInfo {
            reply_to: tx,
            icon_cb: Box::new(icon_cb),
        });
        rx.recv().unwrap_or_default()
    }

    pub fn get_plugin_info(&self, plugin_id: PluginId) -> Option<PluginInfo> {
        let (tx, rx) = std::sync::mpsc::sync_channel::<Option<PluginInfo>>(1);
        let _ = self.sender.send(Cmd::GetPluginInfo {
            id: plugin_id,
            reply_to: tx,
        });

        rx.recv().sure("Failed to respond").flatten()
    }

    pub fn unload(&self, id: PluginId) {
        let _ = self.sender.send(Cmd::UnloadPlugin { id });
    }
}
