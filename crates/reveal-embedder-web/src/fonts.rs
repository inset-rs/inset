//! Web `FontSource`: Roboto as the default face, Noto families for uncovered
//! scripts, loaded as Google Fonts CSS2 subsets (Valo's chunked-family model).
//!
//! Flutter web fetches Noto woff2 slices by unicode-range from gstatic. Here
//! the CSS2 `text=` parameter is that slice: one small woff2 per demand, many
//! faces under one family name.

use std::cell::{Cell, RefCell};
use std::collections::{HashMap, HashSet};
use std::rc::Rc;
use std::sync::Arc;

use reveal_embedder::FontSource;
use valo::{Font, FontAttrs};
use wasm_bindgen::JsCast;

use crate::platform::OnSchedule;

const ROBOTO: &str = "Roboto";
const CSS2: &str = "https://fonts.googleapis.com/css2";

pub struct WebFontSource {
    inner: Rc<Inner>,
}

struct Inner {
    families: RefCell<HashMap<String, Vec<Arc<Vec<u8>>>>>,
    faces: RefCell<Vec<Arc<Vec<u8>>>>,
    pending_families: RefCell<HashSet<String>>,
    pending_codepoints: RefCell<HashSet<char>>,
    inflight: RefCell<HashSet<String>>,
    flush_queued: Cell<bool>,
    fonts_changed: Rc<Cell<bool>>,
    frame_requested: Rc<Cell<bool>>,
    on_schedule: OnSchedule,
}

impl WebFontSource {
    pub fn new(
        fonts_changed: Rc<Cell<bool>>,
        frame_requested: Rc<Cell<bool>>,
        on_schedule: OnSchedule,
    ) -> WebFontSource {
        let inner = Rc::new(Inner {
            families: RefCell::new(HashMap::new()),
            faces: RefCell::new(Vec::new()),
            pending_families: RefCell::new(HashSet::new()),
            pending_codepoints: RefCell::new(HashSet::new()),
            inflight: RefCell::new(HashSet::new()),
            flush_queued: Cell::new(false),
            fonts_changed,
            frame_requested,
            on_schedule,
        });
        inner
            .pending_families
            .borrow_mut()
            .insert(ROBOTO.to_owned());
        queue_flush(&inner);
        WebFontSource { inner }
    }
}

impl FontSource for WebFontSource {
    fn family(&mut self, name: &str) -> Vec<Font> {
        let canonical = canonical_family(name);
        if let Some(files) = self.inner.families.borrow().get(&canonical).cloned() {
            return parse_faces(&files);
        }
        self.inner.pending_families.borrow_mut().insert(canonical);
        queue_flush(&self.inner);
        Vec::new()
    }

    fn face_for_codepoint(&mut self, codepoint: char, _attrs: FontAttrs) -> Option<Font> {
        for file in self.inner.faces.borrow().iter() {
            if let Some(font) = Font::from_data(file.clone(), 0)
                && font.covers(codepoint)
            {
                return Some(font);
            }
        }
        if is_private_use(codepoint) {
            return None;
        }
        self.inner.pending_codepoints.borrow_mut().insert(codepoint);
        queue_flush(&self.inner);
        None
    }
}

fn parse_faces(files: &[Arc<Vec<u8>>]) -> Vec<Font> {
    files
        .iter()
        .filter_map(|file| Font::from_data(file.clone(), 0))
        .collect()
}

fn canonical_family(name: &str) -> String {
    match name {
        "CupertinoSystemText"
        | "CupertinoSystemDisplay"
        | "Arial"
        | "sans-serif"
        | ".SF Pro Text"
        | ".SF Pro Display"
        | ROBOTO => ROBOTO.to_owned(),
        other => other.to_owned(),
    }
}

fn queue_flush(inner: &Rc<Inner>) {
    if inner.flush_queued.replace(true) {
        return;
    }
    let inner = Rc::clone(inner);
    wasm_bindgen_futures::spawn_local(async move {
        inner.flush_queued.set(false);
        flush(&inner).await;
    });
}

async fn flush(inner: &Inner) {
    let families: Vec<String> = inner.pending_families.borrow_mut().drain().collect();
    let codepoints: Vec<char> = inner.pending_codepoints.borrow_mut().drain().collect();
    let mut loaded = false;
    for family in families {
        loaded |= fetch_family(inner, &family, None).await;
    }
    let mut by_family: HashMap<String, String> = HashMap::new();
    for ch in codepoints {
        by_family
            .entry(fallback_family(ch).to_owned())
            .or_default()
            .push(ch);
    }
    for (family, text) in by_family {
        loaded |= fetch_family(inner, &family, Some(&text)).await;
    }
    if loaded {
        inner.fonts_changed.set(true);
        if !inner.frame_requested.replace(true) {
            let callback = inner.on_schedule.borrow().clone();
            if let Some(callback) = callback {
                callback();
            }
        }
    }
}

async fn fetch_family(inner: &Inner, family: &str, text: Option<&str>) -> bool {
    let url = css2_url(family, text);
    if !inner.inflight.borrow_mut().insert(url.clone()) {
        return false;
    }
    let Some(css) = fetch_text(&url).await else {
        return false;
    };
    let mut added = false;
    for font_url in css_font_urls(&css) {
        let Some(bytes) = fetch_bytes(&font_url).await else {
            continue;
        };
        let file = Arc::new(bytes);
        inner
            .families
            .borrow_mut()
            .entry(family.to_owned())
            .or_default()
            .push(Arc::clone(&file));
        inner.faces.borrow_mut().push(file);
        added = true;
    }
    added
}

fn css2_url(family: &str, text: Option<&str>) -> String {
    let mut url = format!("{CSS2}?family={}&display=block", family_query(family));
    if let Some(text) = text {
        url.push_str("&text=");
        url.push_str(&percent_encode(text));
    }
    url
}

fn family_query(family: &str) -> String {
    let encoded = family.replace(' ', "+");
    if family.eq_ignore_ascii_case(ROBOTO) {
        format!("{encoded}:wght@400;500;700")
    } else {
        encoded
    }
}

fn percent_encode(text: &str) -> String {
    let mut out = String::new();
    for byte in text.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(byte as char);
            }
            _ => out.push_str(&format!("%{byte:02X}")),
        }
    }
    out
}

fn css_font_urls(css: &str) -> Vec<String> {
    css.split("url(")
        .skip(1)
        .filter_map(|rest| {
            let rest = rest.trim_start_matches(['\'', '"']);
            let end = rest.find(['\'', '"', ')'])?;
            let url = rest[..end].trim();
            (!url.is_empty()).then(|| url.to_owned())
        })
        .collect()
}

fn fallback_family(ch: char) -> &'static str {
    let c = ch as u32;
    if is_hangul(c) {
        "Noto Sans KR"
    } else if is_kana(c) {
        "Noto Sans JP"
    } else if is_han(c) {
        "Noto Sans SC"
    } else if is_arabic(c) {
        "Noto Sans Arabic"
    } else if (0x0E00..=0x0E7F).contains(&c) {
        "Noto Sans Thai"
    } else if is_emoji(c) {
        "Noto Color Emoji"
    } else {
        "Noto Sans"
    }
}

fn is_hangul(c: u32) -> bool {
    matches!(
        c,
        0x1100..=0x11FF | 0x3130..=0x318F | 0xA960..=0xA97F | 0xAC00..=0xD7FF
    )
}

fn is_kana(c: u32) -> bool {
    matches!(c, 0x3040..=0x30FF | 0x31F0..=0x31FF | 0xFF65..=0xFF9F)
}

fn is_han(c: u32) -> bool {
    matches!(
        c,
        0x2E80..=0x2FFF
            | 0x3000..=0x303F
            | 0x31C0..=0x31EF
            | 0x3400..=0x4DBF
            | 0x4E00..=0x9FFF
            | 0xF900..=0xFAFF
            | 0x20000..=0x2FA1F
    )
}

fn is_arabic(c: u32) -> bool {
    matches!(
        c,
        0x0600..=0x06FF | 0x0750..=0x077F | 0x08A0..=0x08FF | 0xFB50..=0xFDFF | 0xFE70..=0xFEFF
    )
}

fn is_emoji(c: u32) -> bool {
    matches!(
        c,
        0x200D | 0xFE0F | 0x2600..=0x27BF | 0x1F1E6..=0x1F1FF | 0x1F300..=0x1FAFF
    )
}

fn is_private_use(ch: char) -> bool {
    matches!(
        ch,
        '\u{E000}'..='\u{F8FF}' | '\u{F0000}'..='\u{FFFFD}' | '\u{100000}'..='\u{10FFFD}'
    )
}

async fn fetch_text(url: &str) -> Option<String> {
    let bytes = fetch_bytes(url).await?;
    String::from_utf8(bytes).ok()
}

async fn fetch_bytes(url: &str) -> Option<Vec<u8>> {
    let window = web_sys::window()?;
    let response = wasm_bindgen_futures::JsFuture::from(window.fetch_with_str(url))
        .await
        .ok()?;
    let response: web_sys::Response = response.dyn_into().ok()?;
    if !response.ok() {
        return None;
    }
    let buffer = wasm_bindgen_futures::JsFuture::from(response.array_buffer().ok()?)
        .await
        .ok()?;
    Some(js_sys::Uint8Array::new(&buffer).to_vec())
}
