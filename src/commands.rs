use std::{borrow::Cow, sync::Arc};

use crate::commands::fs::IPC_FS;

mod fs;
mod window;

pub(super) trait IPCCommand: Send + Sync {
    fn name(&self) -> &'static str;
    fn respond(
        &self,
        req: &str,
        window_handle: &Arc<tao::window::Window>,
    ) -> Option<Cow<'static, [u8]>>;

    fn is_this(&self, req: &str) -> bool {
        req.starts_with(self.name())
    }

    fn strip_name<'a>(&self, req: &'a str) -> Option<(u32, &'a str)> {
        let mut parts = req.splitn(3, ':');

        let _fn_name = parts.next()?;
        let id_str = parts.next()?;
        let payload = parts.next().unwrap_or("");

        let id = id_str.parse::<u32>().ok()?;
        Some((id, payload))
    }
}

pub fn commands_iter<'a>() -> impl Iterator<Item = &'a dyn IPCCommand> {
    use crate::commands::window::IPC_WINDOW;

    IPC_WINDOW.iter().chain(IPC_FS.iter()).copied()
}

#[macro_export]
macro_rules! ipc_commands {
    (
        $table:ident = [
            $( $struct:ident => $name:literal => $fn:path ),* $(,)?
        ]
    ) => {
        pub(super) static $table: &[&dyn IPCCommand] = &[ $( &$struct ),* ];

        $(
            pub struct $struct;
            impl IPCCommand for $struct {
                fn name(&self) -> &'static str { $name }

                fn respond(
                    &self,
                    req: &str,
                    window_handle: &Arc<tao::window::Window>,
                ) -> Option<std::borrow::Cow<'static, [u8]>> {
                    $fn(req, window_handle)
                }
            }
        )*
    };
}
