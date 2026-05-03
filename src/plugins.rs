#![allow(unused)]

use std::sync::{Arc, mpsc};

use wasmtime::{Instance, TypedFunc};

use crate::{
    AStr,
    plugins::{manifest::PluginManifest, runner::PluginRunner},
    state::samples::{RawSampleEntry, SampleEntrySerialize, SearchRequest},
};

mod host;
mod loader;
mod manifest;
mod runner;

pub(super) use loader::unpack_plugin_zip;

pub use manifest::{PluginId, PluginInfo};

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
        reply_to: mpsc::SyncSender<Result<Vec<SampleEntrySerialize>, String>>,
    },
    UnloadPlugin {
        id: PluginId,
    },
    GetAllPluginsInfo {
        reply_to: mpsc::Sender<Vec<PluginInfo>>,
    },
    Shutdown,
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
            .spawn(move || PluginRunner::new().run(rx))
            .expect("failed to spawn plugin runner thread");

        Self { sender: tx }
    }

    pub fn load(&self, name: &str, bytes: Vec<u8>) {
        let _ = self.sender.send(Cmd::LoadPlugin {
            bytes,
            name: Arc::from(name),
        });
    }

    pub fn search(
        &self,
        id: PluginId,
        req: SearchRequest,
    ) -> Result<Vec<SampleEntrySerialize>, String> {
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

    pub fn shutdown(&self) {
        let _ = self.sender.send(Cmd::Shutdown);
    }
}
