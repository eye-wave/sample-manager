use super::IPCCommand;

mod audio;
mod config;
mod fs;
mod logger;
mod plugins;
mod samples;
mod theme;
mod window;

pub const IPC_ID_BASE: usize = 10;

pub fn commands_iter<'a>() -> impl Iterator<Item = &'a dyn IPCCommand> {
    [
        audio::IPC_AUDIO,
        config::IPC_CONFIG,
        fs::IPC_FS,
        logger::IPC_LOGGER,
        plugins::IPC_PLUGINS,
        samples::IPC_SAMPLES,
        theme::IPC_THEME,
        window::IPC_WINDOW,
    ]
    .into_iter()
    .flatten()
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
            pub(super) static $table: &[&dyn $crate::ipc::IPCCommand] = &[ $( &[<$fn:camel>] ),* ];

            $(
                pub struct [<$fn:camel>];

                impl $crate::ipc::IPCCommand for [<$fn:camel>] {
                    fn name(&self) -> &'static str {
                        stringify!($fn)
                    }

                    fn respond(
                        &self,
                        body: $crate::ipc::IPCBody,
                    ) -> IPCResponse {
                        $fn(body)
                    }
                }
            )*
        }
    };
}

#[cfg(test)]
mod test {
    use std::{fs, path::Path};

    use super::*;

    #[test]
    fn generate_ipc() {
        let mut contents: String = commands_iter()
            .enumerate()
            .map(|(i, c)| {
                format!(
                    "export const {} = {};",
                    c.name().to_uppercase(),
                    i + IPC_ID_BASE
                ) + "\n"
            })
            .collect();

        let max_id = commands_iter().count() + IPC_ID_BASE - 1;

        contents += &format!(
            r#"
type Enumerate<
    N extends number,
    Acc extends number[] = []
> = Acc['length'] extends N
    ? Acc[number]
    : Enumerate<N, [...Acc, Acc['length']]>;

export type Range<F extends number, T extends number> =
    Exclude<Enumerate<T>, Enumerate<F>> | T;

export type IPC_ID = Range<{}, {}>;"#,
            IPC_ID_BASE, max_id
        );

        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("client/src/gen");

        let _ = fs::create_dir(&path);
        let _ = fs::write(path.join("ipc-gen.ts"), contents);
    }
}
