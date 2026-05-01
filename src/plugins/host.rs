use std::collections::HashMap;

use crate::AStr;

pub struct HostState {
    pub storage: HashMap<(AStr, AStr), Vec<u8>>,
}

impl HostState {
    pub(super) fn new() -> Self {
        Self {
            storage: HashMap::new(),
        }
    }

    pub(super) fn is_url_allowed(&self, url: &str, allowlist: &[String]) -> bool {
        let Ok(parsed) = url::Url::parse(url) else {
            return false;
        };
        let Some(host) = parsed.host_str() else {
            return false;
        };
        allowlist
            .iter()
            .any(|allowed| host == allowed || host.ends_with(&format!(".{allowed}")))
    }
}
