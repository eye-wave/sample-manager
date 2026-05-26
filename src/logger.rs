use std::io;

use tracing_subscriber::fmt::MakeWriter;
use tracing_subscriber::fmt::writer::MakeWriterExt;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{EnvFilter, fmt};

use crate::ipc::IPCSenderUI;

mod ansi;

#[derive(Clone)]
struct ChannelWriter {
    tx: IPCSenderUI,
}

impl io::Write for ChannelWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let msg = str::from_utf8(buf)
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "invalid utf-8"))?;

        self.tx.send_msg("log", ansi::ansi_to_html(msg));

        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl<'a> MakeWriter<'a> for ChannelWriter {
    type Writer = Self;

    fn make_writer(&'a self) -> Self::Writer {
        self.clone()
    }
}

pub(super) fn init_logging(tx: IPCSenderUI) {
    let writer = std::io::stdout.and(ChannelWriter { tx });

    tracing_subscriber::registry()
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .with(
            fmt::layer()
                .with_file(true)
                .with_line_number(true)
                .with_ansi(true)
                .with_writer(writer),
        )
        .init();
}
