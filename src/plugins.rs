#![allow(unused)]

use std::sync::{Arc, mpsc};

use wasmtime::{Instance, TypedFunc};

use crate::{
    AStr,
    plugins::{manifest::PluginManifest, runner::PluginRunner},
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
    GetManifest {
        id: AStr,
        reply_to: mpsc::Sender<Option<PluginManifest>>,
    },
    ListPlugins {
        reply_to: mpsc::Sender<Vec<AStr>>,
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

    pub fn get_manifest(&self, id: &str) -> Option<PluginManifest> {
        let (tx, rx) = std::sync::mpsc::channel();

        self.sender
            .send(Cmd::GetManifest {
                id: Arc::from(id),
                reply_to: tx,
            })
            .ok()?;

        rx.recv().ok().flatten()
    }

    pub fn list_all_ids(&self) -> Vec<AStr> {
        let (tx, rx) = std::sync::mpsc::channel();

        if self.sender.send(Cmd::ListPlugins { reply_to: tx }).is_ok() {
            rx.recv().unwrap_or_default()
        } else {
            Vec::new()
        }
    }

    pub fn unload(&self, id: impl Into<AStr>) {
        let _ = self.sender.send(Cmd::UnloadPlugin { id: id.into() });
    }
}
