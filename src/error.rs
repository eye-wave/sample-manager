use std::sync::PoisonError;

pub trait LogErrorExt<T, E> {
    fn sure(self, context: &str) -> Option<T>;
}

impl<T, E: std::fmt::Display> LogErrorExt<T, E> for Result<T, E> {
    fn sure(self, context: &str) -> Option<T> {
        match self {
            Ok(val) => Some(val),
            Err(err) => {
                tracing::error!("{}: {}", context, err);
                None
            }
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum SyncError {
    #[error("mutex poisoned")]
    Poisoned,
}

impl<T> From<PoisonError<T>> for SyncError {
    fn from(_: PoisonError<T>) -> Self {
        SyncError::Poisoned
    }
}
