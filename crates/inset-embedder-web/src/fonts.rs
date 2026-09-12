//! Web `FontSource`: Roboto's Latin slice ships inside the binary as this
//! host's user-interface face, and a character it cannot draw pulls one
//! unicode-range slice of the family carrying its script — Roboto for the
//! scripts Roboto has, a Noto family for the rest — from Google Fonts.
//!
//! Flutter web downloads one Roboto file at start-up and holds the first frame
//! until it lands, then covers missing characters from a table of Noto slices
//! generated ahead of time. Bundling replaces the first half; reading each
//! family's slice table from its CSS2 response on first use replaces the
//! second. The slice files themselves are the ones Flutter downloads.

use std::cell::{Cell, RefCell};
use std::collections::{HashMap, HashSet};
use std::future::Future;
use std::rc::Rc;
use std::sync::Arc;

use inset_embedder::FontSource;
use valo::{Font, FontAttrs};
use wasm_bindgen::JsCast;

use crate::font_fallback::{ROBOTO, fallback_family};
use crate::google_fonts::{Slice, css2_url, parse_slices};
use crate::platform::OnSchedule;

const EMOJI: &str = "Noto Color Emoji";

/// Roboto's Google Fonts Latin slice, variable across weight 100–900, so one
/// file carries every weight a user interface asks for. It ships with the
/// binary because the alternative is a first frame whose text is laid out and
/// spaced but has no ink. See `assets/LICENSE`.
const ROBOTO_LATIN: &[u8] = include_bytes!("../assets/Roboto-Latin.woff2");

/// Flutter's `FallbackFontService` retry policy.
const MAX_ATTEMPTS: u32 = 3;
const RETRY_DELAY_MS: i32 = 1000;

pub struct WebFontSource {
    inner: Rc<Inner>,
}

struct Inner {
    bundled: Arc<Vec<u8>>,
    /// Each family's slice table, keyed by the name the CSS2 API takes.
    families: RefCell<HashMap<&'static str, Family>>,
    /// Slices that have landed, emoji first, searched for a character nothing
    /// registered covers.
    fetched: RefCell<Vec<Fetched>>,
    /// Characters waiting for the next batch of requests.
    pending: RefCell<HashSet<char>>,
    /// Characters no slice can carry (Flutter's `_unsupportedCodePoints`):
    /// asking again would repeat the work on every layout that touches them.
    unsupported: RefCell<HashSet<char>>,
    flush_queued: Cell<bool>,
    notify_queued: Cell<bool>,
    fonts_changed: Rc<Cell<bool>>,
    frame_requested: Rc<Cell<bool>>,
    on_schedule: OnSchedule,
}

enum Family {
    /// The slice table is on its way; these characters wait for it.
    Loading(Vec<char>),
    Ready(Vec<SliceState>),
    /// The table never arrived: nothing of this family can be drawn.
    Unavailable,
}

struct SliceState {
    slice: Slice,
    fetch: Fetch,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Fetch {
    NotStarted,
    Loading,
    Loaded,
    Failed,
}

/// A slice that landed: its file, and its default face for coverage tests.
struct Fetched {
    file: Arc<Vec<u8>>,
    face: Font,
}

impl WebFontSource {
    pub fn new(
        fonts_changed: Rc<Cell<bool>>,
        frame_requested: Rc<Cell<bool>>,
        on_schedule: OnSchedule,
    ) -> WebFontSource {
        WebFontSource {
            inner: Rc::new(Inner {
                bundled: Arc::new(ROBOTO_LATIN.to_vec()),
                families: RefCell::new(HashMap::new()),
                fetched: RefCell::new(Vec::new()),
                pending: RefCell::new(HashSet::new()),
                unsupported: RefCell::new(HashSet::new()),
                flush_queued: Cell::new(false),
                notify_queued: Cell::new(false),
                fonts_changed,
                frame_requested,
                on_schedule,
            }),
        }
    }
}

impl FontSource for WebFontSource {
    fn family(&mut self, name: &str) -> Vec<Font> {
        if name != ROBOTO {
            // Flutter web has a font by no other name either: an application's
            // own fonts are bytes it registers, and every other name misses and
            // resolves through the default family chain.
            return Vec::new();
        }
        Font::instances_from_data(self.inner.bundled.clone(), 0)
    }

    fn face_for_codepoint(&mut self, codepoint: char, attrs: FontAttrs) -> Option<Font> {
        if let Some(face) = self.inner.fetched_face_covering(codepoint, attrs) {
            return Some(face);
        }
        if is_private_use(codepoint) || self.inner.is_unsupported(codepoint) {
            return None;
        }
        self.inner.pending.borrow_mut().insert(codepoint);
        queue_flush(&self.inner);
        None
    }
}

impl Inner {
    fn fetched_face_covering(&self, codepoint: char, attrs: FontAttrs) -> Option<Font> {
        let fetched = self.fetched.borrow();
        let hit = fetched
            .iter()
            .find(|fetched| fetched.face.covers(codepoint))?;
        nearest_instance(&hit.file, attrs)
    }

    fn is_unsupported(&self, codepoint: char) -> bool {
        self.unsupported.borrow().contains(&codepoint)
    }

    fn retire(&self, chars: impl IntoIterator<Item = char>) {
        self.unsupported.borrow_mut().extend(chars);
    }

    /// A landed slice joins the search, emoji ahead of text so a character
    /// both can draw comes out as the emoji — Flutter keeps its emoji family
    /// at the head of the chain for the same reason. `false` when Valo cannot
    /// read the file.
    fn add_fetched(&self, file: Arc<Vec<u8>>, emoji: bool) -> bool {
        let Some(face) = Font::from_data(file.clone(), 0) else {
            return false;
        };
        let mut fetched = self.fetched.borrow_mut();
        let entry = Fetched { file, face };
        if emoji {
            fetched.insert(0, entry);
        } else {
            fetched.push(entry);
        }
        true
    }

    fn slice_url(&self, family: &str, at: usize) -> Option<String> {
        match self.families.borrow().get(family)? {
            Family::Ready(slices) => Some(slices[at].slice.url.clone()),
            _ => None,
        }
    }

    fn set_slice_fetch(&self, family: &str, at: usize, fetch: Fetch) {
        if let Some(Family::Ready(slices)) = self.families.borrow_mut().get_mut(family) {
            slices[at].fetch = fetch;
        }
    }
}

/// The instance of a file nearest `attrs`, so a bold span pulling a fallback
/// gets that font's bold instance rather than its default one.
fn nearest_instance(file: &Arc<Vec<u8>>, attrs: FontAttrs) -> Option<Font> {
    Font::instances_from_data(file.clone(), 0)
        .into_iter()
        .min_by_key(|font| {
            (
                font.attrs().italic != attrs.italic,
                font.attrs().weight.abs_diff(attrs.weight),
            )
        })
}

/// Batches the demands raised during one layout: a pass asks for a character
/// at a time, and grouping them by family finds each family's table once.
fn queue_flush(inner: &Rc<Inner>) {
    if inner.flush_queued.replace(true) {
        return;
    }
    let inner = Rc::clone(inner);
    wasm_bindgen_futures::spawn_local(async move {
        inner.flush_queued.set(false);
        let language = preferred_language();
        let mut by_family: HashMap<&'static str, Vec<char>> = HashMap::new();
        for ch in inner.pending.borrow_mut().drain() {
            by_family
                .entry(fallback_family(ch, &language))
                .or_default()
                .push(ch);
        }
        for (family, chars) in by_family {
            demand(&inner, family, chars);
        }
    });
}

/// Sends characters to their family, fetching its slice table first when this
/// is the family's first character.
fn demand(inner: &Rc<Inner>, family: &'static str, chars: Vec<char>) {
    let unknown = !inner.families.borrow().contains_key(family);
    if unknown {
        inner
            .families
            .borrow_mut()
            .insert(family, Family::Loading(chars));
        wasm_bindgen_futures::spawn_local(fetch_slice_table(Rc::clone(inner), family));
        return;
    }
    route(inner, family, chars);
}

/// Sends each character to the slice carrying it: starts that slice's fetch
/// the first time, and retires the character when no slice has it or when the
/// slice that should is already here or already failed.
fn route(inner: &Rc<Inner>, family: &'static str, chars: Vec<char>) {
    let mut to_fetch = Vec::new();
    let mut retire = Vec::new();
    {
        let mut families = inner.families.borrow_mut();
        match families.get_mut(family) {
            Some(Family::Loading(waiting)) => {
                waiting.extend(chars);
                return;
            }
            Some(Family::Unavailable) | None => retire = chars,
            Some(Family::Ready(slices)) => {
                for ch in chars {
                    let Some(at) = slices.iter().position(|s| s.slice.covers(ch as u32)) else {
                        retire.push(ch);
                        continue;
                    };
                    match slices[at].fetch {
                        Fetch::NotStarted => {
                            slices[at].fetch = Fetch::Loading;
                            to_fetch.push(at);
                        }
                        Fetch::Loading => {}
                        // Here again with the slice in hand means its file does
                        // not carry the character after all.
                        Fetch::Loaded | Fetch::Failed => retire.push(ch),
                    }
                }
            }
        }
    }
    inner.retire(retire);
    for at in to_fetch {
        wasm_bindgen_futures::spawn_local(fetch_slice(Rc::clone(inner), family, at));
    }
}

/// One family's CSS2 response, read into its slice table; the characters that
/// waited on it are routed once it is in.
async fn fetch_slice_table(inner: Rc<Inner>, family: &'static str) {
    let url = css2_url(family, family == ROBOTO);
    let css = with_retries(|| {
        let url = url.clone();
        async move { fetch_text(&url).await }
    })
    .await;
    let slices = css.map(|css| parse_slices(&css)).unwrap_or_default();
    let state = if slices.is_empty() {
        Family::Unavailable
    } else {
        Family::Ready(
            slices
                .into_iter()
                .map(|slice| SliceState {
                    slice,
                    fetch: Fetch::NotStarted,
                })
                .collect(),
        )
    };
    let waiting = match inner.families.borrow_mut().insert(family, state) {
        Some(Family::Loading(waiting)) => waiting,
        _ => Vec::new(),
    };
    route(&inner, family, waiting);
}

/// One slice's file. Either outcome asks text to relayout: a landed slice
/// draws its characters, a failed one retires them on the next demand.
async fn fetch_slice(inner: Rc<Inner>, family: &'static str, at: usize) {
    let Some(url) = inner.slice_url(family, at) else {
        return;
    };
    let bytes = with_retries(|| {
        let url = url.clone();
        async move { fetch_bytes(&url).await }
    })
    .await;
    let landed = bytes.is_some_and(|bytes| inner.add_fetched(Arc::new(bytes), family == EMOJI));
    let fetch = if landed { Fetch::Loaded } else { Fetch::Failed };
    inner.set_slice_fetch(family, at, fetch);
    publish(&inner);
}

/// Flutter's `FallbackFontService` policy: three tries, a second apart.
async fn with_retries<T, Fut>(mut attempt: impl FnMut() -> Fut) -> Option<T>
where
    Fut: Future<Output = Option<T>>,
{
    for n in 1..=MAX_ATTEMPTS {
        if let Some(value) = attempt().await {
            return Some(value);
        }
        if n < MAX_ATTEMPTS {
            sleep(RETRY_DELAY_MS).await;
        }
    }
    None
}

/// Text must relayout against what has landed.
///
/// Deferred to the end of this turn of the event loop, as Flutter defers its
/// own font-change message: several slices can land together, and each would
/// otherwise cost a relayout of its own.
fn publish(inner: &Rc<Inner>) {
    if inner.notify_queued.replace(true) {
        return;
    }
    let inner = Rc::clone(inner);
    wasm_bindgen_futures::spawn_local(async move {
        inner.notify_queued.set(false);
        inner.fonts_changed.set(true);
        if inner.frame_requested.replace(true) {
            return;
        }
        let callback = inner.on_schedule.borrow().clone();
        if let Some(callback) = callback {
            callback();
        }
    });
}

/// The reader's language, which decides the regional face for unified Han —
/// Flutter's `FontFallbackManager` reads the same property.
fn preferred_language() -> String {
    web_sys::window()
        .and_then(|window| window.navigator().language())
        .unwrap_or_default()
}

fn is_private_use(ch: char) -> bool {
    matches!(
        ch,
        '\u{E000}'..='\u{F8FF}' | '\u{F0000}'..='\u{FFFFD}' | '\u{100000}'..='\u{10FFFD}'
    )
}

async fn sleep(millis: i32) {
    let Some(window) = web_sys::window() else {
        return;
    };
    let promise = js_sys::Promise::new(&mut |resolve, _reject| {
        let _ = window.set_timeout_with_callback_and_timeout_and_arguments_0(&resolve, millis);
    });
    let _ = wasm_bindgen_futures::JsFuture::from(promise).await;
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
