//! A video player whose frames the framework draws: the app makes a frame reader and gives it to a
//! player, and a `VideoFrameView` draws the reader's frame at each refresh as part of the picture,
//! at the video's aspect, its corners cut round and turned if asked, like anything painted, with a
//! `SubtitleView` of the player's cues over it. A click plays or pauses it, and a line under it
//! says how far it is and what size its frames are. The player's tracks are printed each time
//! they change.
//!
//! Without an argument it plays Apple's HLS example stream, 1080p at 60 frames a second with a
//! running clock in the picture and subtitle tracks; `--url` plays another link, `--movie` a file,
//! and `--subtitles` shows the first text track in a language, such as `en`.
//!
//! ```sh
//! cargo run -p video_frames
//! cargo run -p video_frames -- --subtitles en
//! cargo run -p video_frames -- --turn 8 --rounded 32
//! cargo run -p video_frames -- --movie ~/Movies/clip.mov
//! ```

#[cfg(target_os = "macos")]
mod app;

/// Apple's example HLS stream for AVFoundation.
#[cfg(target_os = "macos")]
const EXAMPLE_STREAM: &str = concat!(
    "https://devstreaming-cdn.apple.com/videos/streaming/examples/",
    "img_bipbop_adv_example_fmp4/master.m3u8"
);

#[cfg(target_os = "macos")]
fn main() {
    app::run(app::Presentation {
        url: source_url(),
        turn_degrees: number_after("--turn", 0.0),
        corner_radius: number_after("--rounded", 16.0),
        subtitles: argument_after("--subtitles"),
    });
}

#[cfg(not(target_os = "macos"))]
fn main() {
    eprintln!("video_frames: the frames this draws come from macOS's player only");
}

/// The link to play: `--url`, a file's from `--movie`, or Apple's example stream.
#[cfg(target_os = "macos")]
fn source_url() -> String {
    if let Some(url) = argument_after("--url") {
        return url;
    }
    let Some(movie) = argument_after("--movie") else {
        return EXAMPLE_STREAM.to_owned();
    };
    let path = std::fs::canonicalize(&movie).unwrap_or_else(|error| panic!("{movie}: {error}"));
    format!("file://{}", path.display())
}

/// The number after `flag` on the command line, such as `--turn 15`; `otherwise` without it.
#[cfg(target_os = "macos")]
fn number_after(flag: &str, otherwise: f64) -> f64 {
    argument_after(flag).map_or(otherwise, |number| {
        number
            .parse()
            .unwrap_or_else(|_| panic!("{flag} takes a number, such as 15"))
    })
}

/// The command line's argument after `flag`, where it has one.
#[cfg(target_os = "macos")]
fn argument_after(flag: &str) -> Option<String> {
    std::env::args().skip_while(|arg| arg != flag).nth(1)
}
