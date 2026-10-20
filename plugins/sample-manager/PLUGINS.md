# Search plugins

A search plugin is a WebAssembly component built for `wasm32-wasip2`. Every `*.wasm` file in
`<config dir>/sample-manager/plugins` (Linux: `~/.config/sample-manager/plugins`) gets its own entry
in the sidebar; the folder is created on the first run and plugins load at startup.

**Settings -> Plugins** has a card for every file: its icon, name, description and the hosts it may
call. The tick loads / unloads it (an unloaded file stays in the folder), the trash can deletes the
file, **Add Plugin** opens the system's file picker (`.wasm`) and copies the file into the folder.
Each of these loads all the plugins again.

Select a plugin's entry, type in the search box and press **Enter**. Results are not favorites.

## The cache

A plugin only asks the network; it remembers nothing. The app keeps every hit a plugin ever returned
in a binary file of its own (`<data dir>/sample-manager/plugin-data/<plugin>.bin`), loaded at start,
and fuzzy-matches the query against it too (fzf's way: every space-separated word has to match;
rayon spreads it over all cores). So a plugin's tab shows what the network found and then what the
cache has for the query, and works offline: a failing `search` is shown as an error only if the
cache has nothing for the query either. The **Library** tab searches the caches of all plugins as
well as the scanned folders; a cached result is downloaded when clicked, like in its own tab.
Delete the `.bin` to forget a plugin's hits.

## The examples

`plugins-src/` has a Rust plugin:

- `splice`: samples from splice.com, a POST of the website's own GraphQL query (no account, so it
  finds what a logged-out visitor sees). Its files are scrambled; its `unpack` (`splice/src/scramble.rs`)
  unscrambles them after the app downloads them.

```sh
rustup target add wasm32-wasip2
plugins-src/build.sh            # builds it and copies it into the plugins folder
```

## Downloads and dragging into a DAW

Clicking a result downloads the file into the cache (`<cache dir>/sample-manager/downloads`, named
after the sample) and shows it in the player. Once it is downloaded the row can be dragged into a
DAW like any file of the library; a row not downloaded yet is downloaded by the click that starts
the press, the next press drags it. A file that does not decode is deleted again.

A hit with `unpack: true` is not audio when it is downloaded: the app passes the bytes to the
plugin's `unpack(url, data)`, and what that returns is what is kept and dragged. `skip-frames` is a
number of frames at the start of the decoded file that the app leaves out when it shows and plays it
(an encoder delay); the file is not changed. A plugin with plain files sets both to `false` / `0` and
returns `data` from `unpack`.

## Writing one

The interface is `wit/search.wit`; any language that makes components can use it.
In Rust:

```rust
wit_bindgen::generate!({ path: "../../wit", world: "plugin" });
use bx::search::host;
use bx_plugin::icon;
use exports::bx::search::search::{Guest, Hit, Icon, Info};

struct Mine;
impl Guest for Mine {
    fn describe() -> Info {
        Info {
            name: "My source".into(),
            description: "what it is".into(),
            icon: icon!("../icon.rgba", 32), // raw RGBA, see below
            hosts: vec!["example.com".into()], // the only host `http_get` may call
        }
    }
    fn unpack(_url: String, data: Vec<u8>) -> Result<Vec<u8>, String> {
        Ok(data)
    }
    fn search(query: String) -> Result<Vec<Hit>, String> {
        let r = host::http_get("https://example.com/api?q=...", &[])?;
        // parse r.body ...
        Ok(vec![Hit { title: "Kick".into(), url: "https://example.com/kick.wav".into(), ext: None, unpack: false, skip_frames: 0 }])
    }
}
export!(Mine);
```

`Cargo.toml`: `crate-type = ["cdylib"]`, `wit-bindgen = "0.63"`, and `bx-plugin` (`plugins-src/bx-plugin`)
for `icon!`. `search` returns up to 200 hits;
`url` must be http(s); `ext` falls back to the url's extension. An `Err(message)` is shown in the
table when the cache has nothing for the query either. The interface is `bx:search@0.3.0`: version
0.2 had no icon and no `hosts` (a 0.2 plugin has to add them and be rebuilt); version
0.1 had a SQLite database (`db-exec`, `db-script`, `db-query`); those are gone, the host's cache
does that job, so a 0.1 plugin that used them has to drop them and be rebuilt.

| host function                                          | what                                                                             |
| ------------------------------------------------------ | -------------------------------------------------------------------------------- |
| `log(message)`                                         | a line in the app's stderr                                                       |
| `http-get(url, headers) -> response`                   | blocking GET; `status` and `body` (a status of 400 or more is a normal response) |
| `http-request(method, url, headers, body) -> response` | the same with `GET` or `POST` (a body of up to 1 MB)                             |

## Name, icon, permissions

`describe` returns:

- `name`, `description`: shown in the sidebar and on the plugin's card.
- `icon`: raw pixels, no image format: `width`, `height` and `rgba` (`width * height * 4` bytes, sRGB,
  straight alpha, rows top to bottom). It has to be square, 32 to 128 px a side, or the plugin does not
  load. `icon!("icon.rgba", 32)` (crate `bx-plugin`) embeds such a file next to the source file and
  checks its size when the plugin is compiled (the number is both sides, so it cannot be wrong
  shaped).
- `hosts`: the network hosts the plugin may call, names only (`example.com`: exact match, no
  wildcards, no port). `http-get` / `http-request` to any other host fail, and so does a redirect
  that leaves the list (the 3xx answer is returned instead). Empty: no network. While `describe`
  runs the list is still empty. The host's own downloads of the hits' urls are not the plugin's and
  go anywhere.

## Limits

- A component has WASI (so Rust's std works) with no files, no sockets, no environment. Everything
  else goes through the functions above.
- HTTP: http/https only, 15 s timeout, 8 MB response.
- A call over 30 s is interrupted, memory is capped at 256 MB. After a crash or a timeout the plugin
  starts over (fresh instance) on the next search.
- Downloads: 64 MB each, http/https only.
