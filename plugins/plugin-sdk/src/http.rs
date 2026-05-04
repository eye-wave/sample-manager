use crate::host;

const HTTP_BUF: usize = 512 * 1024;

pub struct Request<'a> {
    url: &'a str,
    headers: Vec<(&'a str, &'a str)>,
    body: Option<&'a [u8]>,
}

impl<'a> Request<'a> {
    pub fn get(url: &'a str) -> Self {
        Self {
            url,
            headers: vec![],
            body: None,
        }
    }

    pub fn body(mut self, body: &'a [u8]) -> Self {
        self.body = Some(body);
        self
    }

    pub fn body_str(self, body: &'a str) -> Self {
        self.body(body.as_bytes())
    }

    pub fn json(self, body: &'a str) -> Self {
        self.header("Content-Type", "application/json")
            .body_str(body)
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
        fetch(self.url, &self.headers, self.body)
    }
}

fn fetch(url: &str, headers: &[(&str, &str)], body: Option<&[u8]>) -> Option<Vec<u8>> {
    let mut header_buf = Vec::with_capacity(headers.len() * 16);
    for (k, v) in headers {
        header_buf.extend_from_slice(&(k.as_ptr() as u32).to_le_bytes());
        header_buf.extend_from_slice(&(k.len() as u32).to_le_bytes());
        header_buf.extend_from_slice(&(v.as_ptr() as u32).to_le_bytes());
        header_buf.extend_from_slice(&(v.len() as u32).to_le_bytes());
    }

    let (body_ptr, body_len) = match body {
        Some(b) => (b.as_ptr(), b.len() as u32),
        None => (core::ptr::null(), 0u32),
    };

    let mut out = vec![0u8; HTTP_BUF];
    let n = unsafe {
        host::http_fetch(
            url.as_ptr(),
            url.len() as u32,
            header_buf.as_ptr(),
            headers.len() as u32,
            body_ptr,
            body_len,
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
