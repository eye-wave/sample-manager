#![allow(unused)]

use std::sync::{Arc, mpsc};

use wasmtime::{Instance, TypedFunc};

use crate::{
    AStr,
    plugins::{
        manifest::{PluginInfo, PluginManifest},
        runner::PluginRunner,
    },
    state::samples::SearchRequest,
};

mod host;
mod manifest;
mod runner;

pub struct PluginInstance {
    instance: Instance,
    pub manifest: PluginManifest,
    fn_search: TypedFunc<(u32, u32), u32>,
    fn_alloc: TypedFunc<u32, u32>,
    fn_free: TypedFunc<(u32, u32), ()>,
}

#[derive(Debug)]
pub enum PluginRunnerCommand {
    LoadPlugin {
        name: AStr,
        bytes: Vec<u8>,
    },
    Search {
        id: AStr,
        req: SearchRequest,
    },
    UnloadPlugin {
        id: AStr,
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

    pub fn get_all_plugins_info(&self) -> Vec<PluginInfo> {
        let (tx, rx) = std::sync::mpsc::channel();

        self.sender.send(Cmd::GetAllPluginsInfo { reply_to: tx });

        rx.recv().unwrap_or_default()
    }

    pub fn unload(&self, id: impl Into<AStr>) {
        let _ = self.sender.send(Cmd::UnloadPlugin { id: id.into() });
    }
}
