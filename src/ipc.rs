use crate::commands::{IPCCommand, IPCRequestBody};

mod fs;
mod logger;
mod samples;
mod waveform;
mod window;

pub(super) fn ipc_strip_name<'a>(req: &'a str) -> Option<IPCRequestBody<'a>> {
    let mut parts = req.splitn(3, ':');

    let fn_name = parts.next()?;
    let id_str = parts.next()?;
    let payload = parts.next().unwrap_or("");

    let id = id_str.parse::<u32>().ok()?;
    Some((fn_name, id, payload))
}

pub fn commands_iter<'a>() -> impl Iterator<Item = &'a dyn IPCCommand> {
    use crate::ipc::fs::IPC_FS;
    use crate::ipc::logger::IPC_LOGGER;
    use crate::ipc::samples::IPC_SAMPLES;
    use crate::ipc::waveform::IPC_WAVEFORM;
    use crate::ipc::window::IPC_WINDOW;

    IPC_WINDOW
        .iter()
        .chain(IPC_FS.iter())
        .chain(IPC_LOGGER.iter())
        .chain(IPC_SAMPLES.iter())
        .chain(IPC_WAVEFORM.iter())
        .copied()
}

#[macro_export]
macro_rules! ipc_commands {
    (
        $table:ident = [
            $( $fn:ident ),* $(,)?
        ]
    ) => {
        paste::paste! {
            pub(super) static $table: &[&dyn $crate::commands::IPCCommand] = &[ $( &[<$fn:camel>] ),* ];

            $(
                pub struct [<$fn:camel>];

                impl $crate::commands::IPCCommand for [<$fn:camel>] {
                    fn name(&self) -> &'static str {
                        stringify!($fn)
                    }

                    fn respond(
                        &self,
                        body: $crate::commands::IPCBody
                    ) -> Option<std::borrow::Cow<'static, [u8]>> {
                        $fn(body)
                    }
                }
            )*
        }
    };
}
