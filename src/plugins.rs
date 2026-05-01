use std::sync::{Arc, mpsc};

use wasmtime::{Instance, TypedFunc};

use crate::{
    AStr,
    plugins::{manifest::PluginManifest, runner::PluginRunner},
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
    LoadPlugin { id: AStr, bytes: Vec<u8> },
    UnloadPlugin { id: AStr },
    Shutdown,
}

pub struct PluginRuntimeHandle {
    sender: mpsc::Sender<PluginRunnerCommand>,
}

impl PluginRuntimeHandle {
    pub fn spawn() -> Self {
        let (tx, rx) = mpsc::channel::<PluginRunnerCommand>();

        std::thread::Builder::new()
            .name("plugin-runner".into())
            .spawn(move || PluginRunner::new().run(rx))
            .expect("failed to spawn plugin runner thread");

        Self { sender: tx }
    }

    pub fn load(&self, id: &str, bytes: Vec<u8>) {
        let _ = self.sender.send(PluginRunnerCommand::LoadPlugin {
            id: Arc::from(id),
            bytes,
        });
    }

    pub fn unload(&self, id: impl Into<AStr>) {
        let _ = self
            .sender
            .send(PluginRunnerCommand::UnloadPlugin { id: id.into() });
    }

    pub fn shutdown(&self) {
        let _ = self.sender.send(PluginRunnerCommand::Shutdown);
    }
}
