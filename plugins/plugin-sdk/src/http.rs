use crate::host;

const HTTP_BUF: usize = 512 * 1024;

pub struct Request<'a> {
    url: &'a str,
    headers: Vec<(&'a str, &'a str)>,
}

impl<'a> Request<'a> {
    pub fn get(url: &'a str) -> Self {
        Self {
            url,
            headers: vec![],
        }
    }

    pub fn header(mut self, key: &'a str, value: &'a str) -> Self {
        self.headers.push((key, value));
        self
    }

    pub fn bearer(self, token: &'a str) -> Self {
        // We can't format here without allocation, so bearer is a special case
        // that plugins handle by building the header string themselves.
        // This is a convenience that just calls header().
        self.header("Authorization", token)
    }

    pub fn send(self) -> Option<Vec<u8>> {
        fetch(self.url, &self.headers)
    }

    pub fn send_json(self) -> Option<tinyjson::JsonValue> {
        let bytes = self.send()?;
        let s = std::str::from_utf8(&bytes).ok()?;
        s.parse().ok()
    }
}

fn fetch(url: &str, headers: &[(&str, &str)]) -> Option<Vec<u8>> {
    let mut header_buf = Vec::with_capacity(headers.len() * 16);
    for (k, v) in headers {
        header_buf.extend_from_slice(&(*k as *const str as *const u8 as u32).to_le_bytes());
        header_buf.extend_from_slice(&(k.len() as u32).to_le_bytes());
        header_buf.extend_from_slice(&(*v as *const str as *const u8 as u32).to_le_bytes());
        header_buf.extend_from_slice(&(v.len() as u32).to_le_bytes());
    }

    let mut out = vec![0u8; HTTP_BUF];
    let n = unsafe {
        host::http_fetch(
            url.as_ptr(),
            url.len() as u32,
            header_buf.as_ptr(),
            headers.len() as u32,
            out.as_mut_ptr(),
            out.len() as u32,
        )
    };
    if n < 0 {
        return None;
    }
    out.truncate(n as usize);
    Some(out)
}

pub fn urlencoding(s: &str) -> String {
    s.chars()
        .flat_map(|c| match c {
            'A'..='Z' | 'a'..='z' | '0'..='9' | '-' | '_' | '.' | '~' => vec![c],
            c => format!("%{:02X}", c as u32).chars().collect(),
        })
        .collect()
}
