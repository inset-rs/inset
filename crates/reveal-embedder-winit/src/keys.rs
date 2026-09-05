//! winit key events as Flutter key codes.
//!
//! winit's `KeyCode` and `NamedKey` variants are the W3C UI Events `code` and
//! `key` values, which is what Flutter's engine keys its own tables by: the
//! tables below are transcribed from `dev/tools/gen_keycodes/data/physical_key_data.g.json`
//! and from the web engine's `key_map.g.dart`, and the lookup order follows that
//! engine's `KeyboardConverter._handleEvent`. Machine-written; edit the generator.

use winit::keyboard::{Key, KeyCode, KeyLocation, NamedKey, PhysicalKey};

/// The USB HID usage of a physical key, Flutter's `KeyData.physical`.
///
/// `None` for a key winit reports but Flutter's table has no usage for (the
/// phone-keypad `NumpadHash` / `NumpadStar`, `F25` and up, the legacy Japanese
/// `Hiragana` / `Katakana`), and for a key winit could not identify.
pub(crate) fn physical_key_usage(physical_key: PhysicalKey) -> Option<u64> {
    let PhysicalKey::Code(code) = physical_key else {
        return None;
    };
    Some(match code {
        KeyCode::Backquote => 0x00070035,
        KeyCode::Backslash => 0x00070031,
        KeyCode::BracketLeft => 0x0007002f,
        KeyCode::BracketRight => 0x00070030,
        KeyCode::Comma => 0x00070036,
        KeyCode::Digit0 => 0x00070027,
        KeyCode::Digit1 => 0x0007001e,
        KeyCode::Digit2 => 0x0007001f,
        KeyCode::Digit3 => 0x00070020,
        KeyCode::Digit4 => 0x00070021,
        KeyCode::Digit5 => 0x00070022,
        KeyCode::Digit6 => 0x00070023,
        KeyCode::Digit7 => 0x00070024,
        KeyCode::Digit8 => 0x00070025,
        KeyCode::Digit9 => 0x00070026,
        KeyCode::Equal => 0x0007002e,
        KeyCode::IntlBackslash => 0x00070064,
        KeyCode::IntlRo => 0x00070087,
        KeyCode::IntlYen => 0x00070089,
        KeyCode::KeyA => 0x00070004,
        KeyCode::KeyB => 0x00070005,
        KeyCode::KeyC => 0x00070006,
        KeyCode::KeyD => 0x00070007,
        KeyCode::KeyE => 0x00070008,
        KeyCode::KeyF => 0x00070009,
        KeyCode::KeyG => 0x0007000a,
        KeyCode::KeyH => 0x0007000b,
        KeyCode::KeyI => 0x0007000c,
        KeyCode::KeyJ => 0x0007000d,
        KeyCode::KeyK => 0x0007000e,
        KeyCode::KeyL => 0x0007000f,
        KeyCode::KeyM => 0x00070010,
        KeyCode::KeyN => 0x00070011,
        KeyCode::KeyO => 0x00070012,
        KeyCode::KeyP => 0x00070013,
        KeyCode::KeyQ => 0x00070014,
        KeyCode::KeyR => 0x00070015,
        KeyCode::KeyS => 0x00070016,
        KeyCode::KeyT => 0x00070017,
        KeyCode::KeyU => 0x00070018,
        KeyCode::KeyV => 0x00070019,
        KeyCode::KeyW => 0x0007001a,
        KeyCode::KeyX => 0x0007001b,
        KeyCode::KeyY => 0x0007001c,
        KeyCode::KeyZ => 0x0007001d,
        KeyCode::Minus => 0x0007002d,
        KeyCode::Period => 0x00070037,
        KeyCode::Quote => 0x00070034,
        KeyCode::Semicolon => 0x00070033,
        KeyCode::Slash => 0x00070038,
        KeyCode::AltLeft => 0x000700e2,
        KeyCode::AltRight => 0x000700e6,
        KeyCode::Backspace => 0x0007002a,
        KeyCode::CapsLock => 0x00070039,
        KeyCode::ContextMenu => 0x00070065,
        KeyCode::ControlLeft => 0x000700e0,
        KeyCode::ControlRight => 0x000700e4,
        KeyCode::Enter => 0x00070028,
        KeyCode::SuperLeft => 0x000700e3,
        KeyCode::SuperRight => 0x000700e7,
        KeyCode::ShiftLeft => 0x000700e1,
        KeyCode::ShiftRight => 0x000700e5,
        KeyCode::Space => 0x0007002c,
        KeyCode::Tab => 0x0007002b,
        KeyCode::Convert => 0x0007008a,
        KeyCode::KanaMode => 0x00070088,
        KeyCode::Lang1 => 0x00070090,
        KeyCode::Lang2 => 0x00070091,
        KeyCode::Lang3 => 0x00070092,
        KeyCode::Lang4 => 0x00070093,
        KeyCode::Lang5 => 0x00070094,
        KeyCode::NonConvert => 0x0007008b,
        KeyCode::Delete => 0x0007004c,
        KeyCode::End => 0x0007004d,
        KeyCode::Help => 0x00070075,
        KeyCode::Home => 0x0007004a,
        KeyCode::Insert => 0x00070049,
        KeyCode::PageDown => 0x0007004e,
        KeyCode::PageUp => 0x0007004b,
        KeyCode::ArrowDown => 0x00070051,
        KeyCode::ArrowLeft => 0x00070050,
        KeyCode::ArrowRight => 0x0007004f,
        KeyCode::ArrowUp => 0x00070052,
        KeyCode::NumLock => 0x00070053,
        KeyCode::Numpad0 => 0x00070062,
        KeyCode::Numpad1 => 0x00070059,
        KeyCode::Numpad2 => 0x0007005a,
        KeyCode::Numpad3 => 0x0007005b,
        KeyCode::Numpad4 => 0x0007005c,
        KeyCode::Numpad5 => 0x0007005d,
        KeyCode::Numpad6 => 0x0007005e,
        KeyCode::Numpad7 => 0x0007005f,
        KeyCode::Numpad8 => 0x00070060,
        KeyCode::Numpad9 => 0x00070061,
        KeyCode::NumpadAdd => 0x00070057,
        KeyCode::NumpadBackspace => 0x000700bb,
        KeyCode::NumpadClear => 0x000700d8,
        KeyCode::NumpadClearEntry => 0x000700d9,
        KeyCode::NumpadComma => 0x00070085,
        KeyCode::NumpadDecimal => 0x00070063,
        KeyCode::NumpadDivide => 0x00070054,
        KeyCode::NumpadEnter => 0x00070058,
        KeyCode::NumpadEqual => 0x00070067,
        KeyCode::NumpadMemoryAdd => 0x000700d3,
        KeyCode::NumpadMemoryClear => 0x000700d2,
        KeyCode::NumpadMemoryRecall => 0x000700d1,
        KeyCode::NumpadMemoryStore => 0x000700d0,
        KeyCode::NumpadMemorySubtract => 0x000700d4,
        KeyCode::NumpadMultiply => 0x00070055,
        KeyCode::NumpadParenLeft => 0x000700b6,
        KeyCode::NumpadParenRight => 0x000700b7,
        KeyCode::NumpadSubtract => 0x00070056,
        KeyCode::Escape => 0x00070029,
        KeyCode::Fn => 0x00000012,
        KeyCode::FnLock => 0x00000013,
        KeyCode::PrintScreen => 0x00070046,
        KeyCode::ScrollLock => 0x00070047,
        KeyCode::Pause => 0x00070048,
        KeyCode::BrowserBack => 0x000c0224,
        KeyCode::BrowserFavorites => 0x000c022a,
        KeyCode::BrowserForward => 0x000c0225,
        KeyCode::BrowserHome => 0x000c0223,
        KeyCode::BrowserRefresh => 0x000c0227,
        KeyCode::BrowserSearch => 0x000c0221,
        KeyCode::BrowserStop => 0x000c0226,
        KeyCode::Eject => 0x000c00b8,
        KeyCode::LaunchApp1 => 0x000c0194,
        KeyCode::LaunchApp2 => 0x000c0192,
        KeyCode::LaunchMail => 0x000c018a,
        KeyCode::MediaPlayPause => 0x000c00cd,
        KeyCode::MediaSelect => 0x000c0183,
        KeyCode::MediaStop => 0x000c00b7,
        KeyCode::MediaTrackNext => 0x000c00b5,
        KeyCode::MediaTrackPrevious => 0x000c00b6,
        KeyCode::Power => 0x00070066,
        KeyCode::Sleep => 0x00010082,
        KeyCode::AudioVolumeDown => 0x00070081,
        KeyCode::AudioVolumeMute => 0x0007007f,
        KeyCode::AudioVolumeUp => 0x00070080,
        KeyCode::WakeUp => 0x00010083,
        KeyCode::Meta => 0x00000011,
        KeyCode::Hyper => 0x00000010,
        KeyCode::Turbo => 0x00000016,
        KeyCode::Abort => 0x0007009b,
        KeyCode::Resume => 0x00000015,
        KeyCode::Suspend => 0x00000014,
        KeyCode::Again => 0x00070079,
        KeyCode::Copy => 0x0007007c,
        KeyCode::Cut => 0x0007007b,
        KeyCode::Find => 0x0007007e,
        KeyCode::Open => 0x00070074,
        KeyCode::Paste => 0x0007007d,
        KeyCode::Props => 0x000700a3,
        KeyCode::Select => 0x00070077,
        KeyCode::Undo => 0x0007007a,
        KeyCode::F1 => 0x0007003a,
        KeyCode::F2 => 0x0007003b,
        KeyCode::F3 => 0x0007003c,
        KeyCode::F4 => 0x0007003d,
        KeyCode::F5 => 0x0007003e,
        KeyCode::F6 => 0x0007003f,
        KeyCode::F7 => 0x00070040,
        KeyCode::F8 => 0x00070041,
        KeyCode::F9 => 0x00070042,
        KeyCode::F10 => 0x00070043,
        KeyCode::F11 => 0x00070044,
        KeyCode::F12 => 0x00070045,
        KeyCode::F13 => 0x00070068,
        KeyCode::F14 => 0x00070069,
        KeyCode::F15 => 0x0007006a,
        KeyCode::F16 => 0x0007006b,
        KeyCode::F17 => 0x0007006c,
        KeyCode::F18 => 0x0007006d,
        KeyCode::F19 => 0x0007006e,
        KeyCode::F20 => 0x0007006f,
        KeyCode::F21 => 0x00070070,
        KeyCode::F22 => 0x00070071,
        KeyCode::F23 => 0x00070072,
        KeyCode::F24 => 0x00070073,
        _ => return None,
    })
}

/// The logical key id of a key press, Flutter's `KeyData.logical`.
///
/// The lookup order is the web engine's: the table of named keys, then the table
/// of keys whose id depends on their location, then — for a key that produces a
/// character — the character's own code point in the Unicode plane.
pub(crate) fn logical_key_id(key: &Key, location: KeyLocation) -> Option<u64> {
    let value = match key {
        Key::Named(named) => named_key_value(*named),
        Key::Character(text) => text.as_str(),
        Key::Dead(_) | Key::Unidentified(_) => return None,
    };
    if let Some(id) = web_to_logical(value) {
        return Some(id);
    }
    if let Some(id) = logical_location(value, location) {
        return Some(id);
    }
    if event_key_is_key_name(value) {
        return None;
    }
    character_logical_key(value)
}

/// The character a key event produced, Flutter's `KeyData.character`.
///
/// Control characters are dropped, as Flutter's embedders drop them: a key
/// without a visual representation produces no character. Up events carry none;
/// that is the caller's part, as in the engine.
pub(crate) fn character_of(text: Option<&str>) -> Option<String> {
    text.filter(|text| !is_control_character(text))
        .map(str::to_owned)
}

/// Flutter's `LogicalKeyboardKey.isControlCharacter`, which its embedders use to
/// keep a control character out of `KeyEvent.character`. The framework's copy is
/// in `reveal-services`; a host crate does not depend on the framework.
fn is_control_character(label: &str) -> bool {
    let mut characters = label.chars();
    let (Some(code_unit), None) = (characters.next(), characters.next()) else {
        return false;
    };
    let code_unit = u32::from(code_unit);
    code_unit <= 0x1f || (0x7f..=0x9f).contains(&code_unit)
}

/// Whether the W3C `key` value is a key name, such as "Shift", rather than a
/// character, such as "S" or "ж".
///
/// A key name always has more than 1 code unit, and they are all alnums.
/// Character keys, however, can also have more than 1 code unit: en-in maps KeyL
/// to L̥/l̥. To resolve this, the second code unit decides.
fn event_key_is_key_name(key: &str) -> bool {
    let mut code_units = key.encode_utf16();
    match (code_units.next(), code_units.next()) {
        (Some(first), Some(second)) => first < 0x7f && second < 0x7f,
        _ => false,
    }
}

/// A character's logical key: its lower case code point, in Flutter's Unicode
/// plane, which is how Flutter numbers every character-producing key.
fn character_logical_key(value: &str) -> Option<u64> {
    let character = value.chars().next()?;
    let mut lowercase = character.to_lowercase();
    match (lowercase.next(), lowercase.next()) {
        (Some(lowered), None) => Some(u64::from(lowered)),
        _ => Some(u64::from(character)),
    }
}

/// The web engine's `kWebToLogicalKey`.
fn web_to_logical(value: &str) -> Option<u64> {
    WEB_TO_LOGICAL
        .binary_search_by_key(&value, |&(name, _)| name)
        .ok()
        .map(|index| WEB_TO_LOGICAL[index].1)
}

/// The web engine's `kWebLogicalLocationMap`, indexed by the DOM key location
/// winit reports as [`KeyLocation`].
fn logical_location(value: &str, location: KeyLocation) -> Option<u64> {
    let index = match location {
        KeyLocation::Standard => 0,
        KeyLocation::Left => 1,
        KeyLocation::Right => 2,
        KeyLocation::Numpad => 3,
    };
    LOGICAL_LOCATION
        .binary_search_by_key(&value, |&(name, _)| name)
        .ok()
        .and_then(|found| LOGICAL_LOCATION[found].1[index])
}

/// The W3C `key` value of a named key.
fn named_key_value(key: NamedKey) -> &'static str {
    match key {
        NamedKey::Alt => "Alt",
        NamedKey::AltGraph => "AltGraph",
        NamedKey::CapsLock => "CapsLock",
        NamedKey::Control => "Control",
        NamedKey::Fn => "Fn",
        NamedKey::FnLock => "FnLock",
        NamedKey::NumLock => "NumLock",
        NamedKey::ScrollLock => "ScrollLock",
        NamedKey::Shift => "Shift",
        NamedKey::Symbol => "Symbol",
        NamedKey::SymbolLock => "SymbolLock",
        NamedKey::Meta => "Meta",
        NamedKey::Hyper => "Hyper",
        NamedKey::Super => "Super",
        NamedKey::Enter => "Enter",
        NamedKey::Tab => "Tab",
        NamedKey::Space => " ",
        NamedKey::ArrowDown => "ArrowDown",
        NamedKey::ArrowLeft => "ArrowLeft",
        NamedKey::ArrowRight => "ArrowRight",
        NamedKey::ArrowUp => "ArrowUp",
        NamedKey::End => "End",
        NamedKey::Home => "Home",
        NamedKey::PageDown => "PageDown",
        NamedKey::PageUp => "PageUp",
        NamedKey::Backspace => "Backspace",
        NamedKey::Clear => "Clear",
        NamedKey::Copy => "Copy",
        NamedKey::CrSel => "CrSel",
        NamedKey::Cut => "Cut",
        NamedKey::Delete => "Delete",
        NamedKey::EraseEof => "EraseEof",
        NamedKey::ExSel => "ExSel",
        NamedKey::Insert => "Insert",
        NamedKey::Paste => "Paste",
        NamedKey::Redo => "Redo",
        NamedKey::Undo => "Undo",
        NamedKey::Accept => "Accept",
        NamedKey::Again => "Again",
        NamedKey::Attn => "Attn",
        NamedKey::Cancel => "Cancel",
        NamedKey::ContextMenu => "ContextMenu",
        NamedKey::Escape => "Escape",
        NamedKey::Execute => "Execute",
        NamedKey::Find => "Find",
        NamedKey::Help => "Help",
        NamedKey::Pause => "Pause",
        NamedKey::Play => "Play",
        NamedKey::Props => "Props",
        NamedKey::Select => "Select",
        NamedKey::ZoomIn => "ZoomIn",
        NamedKey::ZoomOut => "ZoomOut",
        NamedKey::BrightnessDown => "BrightnessDown",
        NamedKey::BrightnessUp => "BrightnessUp",
        NamedKey::Eject => "Eject",
        NamedKey::LogOff => "LogOff",
        NamedKey::Power => "Power",
        NamedKey::PowerOff => "PowerOff",
        NamedKey::PrintScreen => "PrintScreen",
        NamedKey::Hibernate => "Hibernate",
        NamedKey::Standby => "Standby",
        NamedKey::WakeUp => "WakeUp",
        NamedKey::AllCandidates => "AllCandidates",
        NamedKey::Alphanumeric => "Alphanumeric",
        NamedKey::CodeInput => "CodeInput",
        NamedKey::Compose => "Compose",
        NamedKey::Convert => "Convert",
        NamedKey::FinalMode => "FinalMode",
        NamedKey::GroupFirst => "GroupFirst",
        NamedKey::GroupLast => "GroupLast",
        NamedKey::GroupNext => "GroupNext",
        NamedKey::GroupPrevious => "GroupPrevious",
        NamedKey::ModeChange => "ModeChange",
        NamedKey::NextCandidate => "NextCandidate",
        NamedKey::NonConvert => "NonConvert",
        NamedKey::PreviousCandidate => "PreviousCandidate",
        NamedKey::Process => "Process",
        NamedKey::SingleCandidate => "SingleCandidate",
        NamedKey::HangulMode => "HangulMode",
        NamedKey::HanjaMode => "HanjaMode",
        NamedKey::JunjaMode => "JunjaMode",
        NamedKey::Eisu => "Eisu",
        NamedKey::Hankaku => "Hankaku",
        NamedKey::Hiragana => "Hiragana",
        NamedKey::HiraganaKatakana => "HiraganaKatakana",
        NamedKey::KanaMode => "KanaMode",
        NamedKey::KanjiMode => "KanjiMode",
        NamedKey::Katakana => "Katakana",
        NamedKey::Romaji => "Romaji",
        NamedKey::Zenkaku => "Zenkaku",
        NamedKey::ZenkakuHankaku => "ZenkakuHankaku",
        NamedKey::Soft1 => "Soft1",
        NamedKey::Soft2 => "Soft2",
        NamedKey::Soft3 => "Soft3",
        NamedKey::Soft4 => "Soft4",
        NamedKey::ChannelDown => "ChannelDown",
        NamedKey::ChannelUp => "ChannelUp",
        NamedKey::Close => "Close",
        NamedKey::MailForward => "MailForward",
        NamedKey::MailReply => "MailReply",
        NamedKey::MailSend => "MailSend",
        NamedKey::MediaClose => "MediaClose",
        NamedKey::MediaFastForward => "MediaFastForward",
        NamedKey::MediaPause => "MediaPause",
        NamedKey::MediaPlay => "MediaPlay",
        NamedKey::MediaPlayPause => "MediaPlayPause",
        NamedKey::MediaRecord => "MediaRecord",
        NamedKey::MediaRewind => "MediaRewind",
        NamedKey::MediaStop => "MediaStop",
        NamedKey::MediaTrackNext => "MediaTrackNext",
        NamedKey::MediaTrackPrevious => "MediaTrackPrevious",
        NamedKey::New => "New",
        NamedKey::Open => "Open",
        NamedKey::Print => "Print",
        NamedKey::Save => "Save",
        NamedKey::SpellCheck => "SpellCheck",
        NamedKey::Key11 => "Key11",
        NamedKey::Key12 => "Key12",
        NamedKey::AudioBalanceLeft => "AudioBalanceLeft",
        NamedKey::AudioBalanceRight => "AudioBalanceRight",
        NamedKey::AudioBassBoostDown => "AudioBassBoostDown",
        NamedKey::AudioBassBoostToggle => "AudioBassBoostToggle",
        NamedKey::AudioBassBoostUp => "AudioBassBoostUp",
        NamedKey::AudioFaderFront => "AudioFaderFront",
        NamedKey::AudioFaderRear => "AudioFaderRear",
        NamedKey::AudioSurroundModeNext => "AudioSurroundModeNext",
        NamedKey::AudioTrebleDown => "AudioTrebleDown",
        NamedKey::AudioTrebleUp => "AudioTrebleUp",
        NamedKey::AudioVolumeDown => "AudioVolumeDown",
        NamedKey::AudioVolumeUp => "AudioVolumeUp",
        NamedKey::AudioVolumeMute => "AudioVolumeMute",
        NamedKey::MicrophoneToggle => "MicrophoneToggle",
        NamedKey::MicrophoneVolumeDown => "MicrophoneVolumeDown",
        NamedKey::MicrophoneVolumeUp => "MicrophoneVolumeUp",
        NamedKey::MicrophoneVolumeMute => "MicrophoneVolumeMute",
        NamedKey::SpeechCorrectionList => "SpeechCorrectionList",
        NamedKey::SpeechInputToggle => "SpeechInputToggle",
        NamedKey::LaunchApplication1 => "LaunchApplication1",
        NamedKey::LaunchApplication2 => "LaunchApplication2",
        NamedKey::LaunchCalendar => "LaunchCalendar",
        NamedKey::LaunchContacts => "LaunchContacts",
        NamedKey::LaunchMail => "LaunchMail",
        NamedKey::LaunchMediaPlayer => "LaunchMediaPlayer",
        NamedKey::LaunchMusicPlayer => "LaunchMusicPlayer",
        NamedKey::LaunchPhone => "LaunchPhone",
        NamedKey::LaunchScreenSaver => "LaunchScreenSaver",
        NamedKey::LaunchSpreadsheet => "LaunchSpreadsheet",
        NamedKey::LaunchWebBrowser => "LaunchWebBrowser",
        NamedKey::LaunchWebCam => "LaunchWebCam",
        NamedKey::LaunchWordProcessor => "LaunchWordProcessor",
        NamedKey::BrowserBack => "BrowserBack",
        NamedKey::BrowserFavorites => "BrowserFavorites",
        NamedKey::BrowserForward => "BrowserForward",
        NamedKey::BrowserHome => "BrowserHome",
        NamedKey::BrowserRefresh => "BrowserRefresh",
        NamedKey::BrowserSearch => "BrowserSearch",
        NamedKey::BrowserStop => "BrowserStop",
        NamedKey::AppSwitch => "AppSwitch",
        NamedKey::Call => "Call",
        NamedKey::Camera => "Camera",
        NamedKey::CameraFocus => "CameraFocus",
        NamedKey::EndCall => "EndCall",
        NamedKey::GoBack => "GoBack",
        NamedKey::GoHome => "GoHome",
        NamedKey::HeadsetHook => "HeadsetHook",
        NamedKey::LastNumberRedial => "LastNumberRedial",
        NamedKey::Notification => "Notification",
        NamedKey::MannerMode => "MannerMode",
        NamedKey::VoiceDial => "VoiceDial",
        NamedKey::TV => "TV",
        NamedKey::TV3DMode => "TV3DMode",
        NamedKey::TVAntennaCable => "TVAntennaCable",
        NamedKey::TVAudioDescription => "TVAudioDescription",
        NamedKey::TVAudioDescriptionMixDown => "TVAudioDescriptionMixDown",
        NamedKey::TVAudioDescriptionMixUp => "TVAudioDescriptionMixUp",
        NamedKey::TVContentsMenu => "TVContentsMenu",
        NamedKey::TVDataService => "TVDataService",
        NamedKey::TVInput => "TVInput",
        NamedKey::TVInputComponent1 => "TVInputComponent1",
        NamedKey::TVInputComponent2 => "TVInputComponent2",
        NamedKey::TVInputComposite1 => "TVInputComposite1",
        NamedKey::TVInputComposite2 => "TVInputComposite2",
        NamedKey::TVInputHDMI1 => "TVInputHDMI1",
        NamedKey::TVInputHDMI2 => "TVInputHDMI2",
        NamedKey::TVInputHDMI3 => "TVInputHDMI3",
        NamedKey::TVInputHDMI4 => "TVInputHDMI4",
        NamedKey::TVInputVGA1 => "TVInputVGA1",
        NamedKey::TVMediaContext => "TVMediaContext",
        NamedKey::TVNetwork => "TVNetwork",
        NamedKey::TVNumberEntry => "TVNumberEntry",
        NamedKey::TVPower => "TVPower",
        NamedKey::TVRadioService => "TVRadioService",
        NamedKey::TVSatellite => "TVSatellite",
        NamedKey::TVSatelliteBS => "TVSatelliteBS",
        NamedKey::TVSatelliteCS => "TVSatelliteCS",
        NamedKey::TVSatelliteToggle => "TVSatelliteToggle",
        NamedKey::TVTerrestrialAnalog => "TVTerrestrialAnalog",
        NamedKey::TVTerrestrialDigital => "TVTerrestrialDigital",
        NamedKey::TVTimer => "TVTimer",
        NamedKey::AVRInput => "AVRInput",
        NamedKey::AVRPower => "AVRPower",
        NamedKey::ColorF0Red => "ColorF0Red",
        NamedKey::ColorF1Green => "ColorF1Green",
        NamedKey::ColorF2Yellow => "ColorF2Yellow",
        NamedKey::ColorF3Blue => "ColorF3Blue",
        NamedKey::ColorF4Grey => "ColorF4Grey",
        NamedKey::ColorF5Brown => "ColorF5Brown",
        NamedKey::ClosedCaptionToggle => "ClosedCaptionToggle",
        NamedKey::Dimmer => "Dimmer",
        NamedKey::DisplaySwap => "DisplaySwap",
        NamedKey::DVR => "DVR",
        NamedKey::Exit => "Exit",
        NamedKey::FavoriteClear0 => "FavoriteClear0",
        NamedKey::FavoriteClear1 => "FavoriteClear1",
        NamedKey::FavoriteClear2 => "FavoriteClear2",
        NamedKey::FavoriteClear3 => "FavoriteClear3",
        NamedKey::FavoriteRecall0 => "FavoriteRecall0",
        NamedKey::FavoriteRecall1 => "FavoriteRecall1",
        NamedKey::FavoriteRecall2 => "FavoriteRecall2",
        NamedKey::FavoriteRecall3 => "FavoriteRecall3",
        NamedKey::FavoriteStore0 => "FavoriteStore0",
        NamedKey::FavoriteStore1 => "FavoriteStore1",
        NamedKey::FavoriteStore2 => "FavoriteStore2",
        NamedKey::FavoriteStore3 => "FavoriteStore3",
        NamedKey::Guide => "Guide",
        NamedKey::GuideNextDay => "GuideNextDay",
        NamedKey::GuidePreviousDay => "GuidePreviousDay",
        NamedKey::Info => "Info",
        NamedKey::InstantReplay => "InstantReplay",
        NamedKey::Link => "Link",
        NamedKey::ListProgram => "ListProgram",
        NamedKey::LiveContent => "LiveContent",
        NamedKey::Lock => "Lock",
        NamedKey::MediaApps => "MediaApps",
        NamedKey::MediaAudioTrack => "MediaAudioTrack",
        NamedKey::MediaLast => "MediaLast",
        NamedKey::MediaSkipBackward => "MediaSkipBackward",
        NamedKey::MediaSkipForward => "MediaSkipForward",
        NamedKey::MediaStepBackward => "MediaStepBackward",
        NamedKey::MediaStepForward => "MediaStepForward",
        NamedKey::MediaTopMenu => "MediaTopMenu",
        NamedKey::NavigateIn => "NavigateIn",
        NamedKey::NavigateNext => "NavigateNext",
        NamedKey::NavigateOut => "NavigateOut",
        NamedKey::NavigatePrevious => "NavigatePrevious",
        NamedKey::NextFavoriteChannel => "NextFavoriteChannel",
        NamedKey::NextUserProfile => "NextUserProfile",
        NamedKey::OnDemand => "OnDemand",
        NamedKey::Pairing => "Pairing",
        NamedKey::PinPDown => "PinPDown",
        NamedKey::PinPMove => "PinPMove",
        NamedKey::PinPToggle => "PinPToggle",
        NamedKey::PinPUp => "PinPUp",
        NamedKey::PlaySpeedDown => "PlaySpeedDown",
        NamedKey::PlaySpeedReset => "PlaySpeedReset",
        NamedKey::PlaySpeedUp => "PlaySpeedUp",
        NamedKey::RandomToggle => "RandomToggle",
        NamedKey::RcLowBattery => "RcLowBattery",
        NamedKey::RecordSpeedNext => "RecordSpeedNext",
        NamedKey::RfBypass => "RfBypass",
        NamedKey::ScanChannelsToggle => "ScanChannelsToggle",
        NamedKey::ScreenModeNext => "ScreenModeNext",
        NamedKey::Settings => "Settings",
        NamedKey::SplitScreenToggle => "SplitScreenToggle",
        NamedKey::STBInput => "STBInput",
        NamedKey::STBPower => "STBPower",
        NamedKey::Subtitle => "Subtitle",
        NamedKey::Teletext => "Teletext",
        NamedKey::VideoModeNext => "VideoModeNext",
        NamedKey::Wink => "Wink",
        NamedKey::ZoomToggle => "ZoomToggle",
        NamedKey::F1 => "F1",
        NamedKey::F2 => "F2",
        NamedKey::F3 => "F3",
        NamedKey::F4 => "F4",
        NamedKey::F5 => "F5",
        NamedKey::F6 => "F6",
        NamedKey::F7 => "F7",
        NamedKey::F8 => "F8",
        NamedKey::F9 => "F9",
        NamedKey::F10 => "F10",
        NamedKey::F11 => "F11",
        NamedKey::F12 => "F12",
        NamedKey::F13 => "F13",
        NamedKey::F14 => "F14",
        NamedKey::F15 => "F15",
        NamedKey::F16 => "F16",
        NamedKey::F17 => "F17",
        NamedKey::F18 => "F18",
        NamedKey::F19 => "F19",
        NamedKey::F20 => "F20",
        NamedKey::F21 => "F21",
        NamedKey::F22 => "F22",
        NamedKey::F23 => "F23",
        NamedKey::F24 => "F24",
        NamedKey::F25 => "F25",
        NamedKey::F26 => "F26",
        NamedKey::F27 => "F27",
        NamedKey::F28 => "F28",
        NamedKey::F29 => "F29",
        NamedKey::F30 => "F30",
        NamedKey::F31 => "F31",
        NamedKey::F32 => "F32",
        NamedKey::F33 => "F33",
        NamedKey::F34 => "F34",
        NamedKey::F35 => "F35",
        _ => "",
    }
}

static WEB_TO_LOGICAL: &[(&str, u64)] = &[
    ("AVRInput", 0x00100000d08),
    ("AVRPower", 0x00100000d09),
    ("Accel", 0x00100000101),
    ("Accept", 0x00100000501),
    ("Again", 0x00100000502),
    ("AllCandidates", 0x00100000701),
    ("Alphanumeric", 0x00100000702),
    ("AltGraph", 0x00100000103),
    ("AppSwitch", 0x00100001001),
    ("ArrowDown", 0x00100000301),
    ("ArrowLeft", 0x00100000302),
    ("ArrowRight", 0x00100000303),
    ("ArrowUp", 0x00100000304),
    ("Attn", 0x00100000503),
    ("AudioBalanceLeft", 0x00100000d01),
    ("AudioBalanceRight", 0x00100000d02),
    ("AudioBassBoostDown", 0x00100000d03),
    ("AudioBassBoostToggle", 0x00100000e02),
    ("AudioBassBoostUp", 0x00100000d04),
    ("AudioFaderFront", 0x00100000d05),
    ("AudioFaderRear", 0x00100000d06),
    ("AudioSurroundModeNext", 0x00100000d07),
    ("AudioTrebleDown", 0x00100000e04),
    ("AudioTrebleUp", 0x00100000e05),
    ("AudioVolumeDown", 0x00100000a0f),
    ("AudioVolumeMute", 0x00100000a11),
    ("AudioVolumeUp", 0x00100000a10),
    ("Backspace", 0x00100000008),
    ("BrightnessDown", 0x00100000601),
    ("BrightnessUp", 0x00100000602),
    ("BrowserBack", 0x00100000c01),
    ("BrowserFavorites", 0x00100000c02),
    ("BrowserForward", 0x00100000c03),
    ("BrowserHome", 0x00100000c04),
    ("BrowserRefresh", 0x00100000c05),
    ("BrowserSearch", 0x00100000c06),
    ("BrowserStop", 0x00100000c07),
    ("Call", 0x00100001002),
    ("Camera", 0x00100000603),
    ("CameraFocus", 0x00100001003),
    ("Cancel", 0x00100000504),
    ("CapsLock", 0x00100000104),
    ("ChannelDown", 0x00100000d0a),
    ("ChannelUp", 0x00100000d0b),
    ("Clear", 0x00100000401),
    ("Close", 0x00100000a01),
    ("ClosedCaptionToggle", 0x00100000d12),
    ("CodeInput", 0x00100000703),
    ("ColorF0Red", 0x00100000d0c),
    ("ColorF1Green", 0x00100000d0d),
    ("ColorF2Yellow", 0x00100000d0e),
    ("ColorF3Blue", 0x00100000d0f),
    ("ColorF4Grey", 0x00100000d10),
    ("ColorF5Brown", 0x00100000d11),
    ("Compose", 0x00100000704),
    ("ContextMenu", 0x00100000505),
    ("Convert", 0x00100000705),
    ("Copy", 0x00100000402),
    ("CrSel", 0x00100000403),
    ("Cut", 0x00100000404),
    ("DVR", 0x00100000d4f),
    ("Delete", 0x0010000007f),
    ("Dimmer", 0x00100000d13),
    ("DisplaySwap", 0x00100000d14),
    ("Eisu", 0x00100000714),
    ("Eject", 0x00100000604),
    ("End", 0x00100000305),
    ("EndCall", 0x00100001004),
    ("Enter", 0x0010000000d),
    ("EraseEof", 0x00100000405),
    ("Esc", 0x0010000001b),
    ("Escape", 0x0010000001b),
    ("ExSel", 0x00100000406),
    ("Execute", 0x00100000506),
    ("Exit", 0x00100000d15),
    ("F1", 0x00100000801),
    ("F10", 0x0010000080a),
    ("F11", 0x0010000080b),
    ("F12", 0x0010000080c),
    ("F13", 0x0010000080d),
    ("F14", 0x0010000080e),
    ("F15", 0x0010000080f),
    ("F16", 0x00100000810),
    ("F17", 0x00100000811),
    ("F18", 0x00100000812),
    ("F19", 0x00100000813),
    ("F2", 0x00100000802),
    ("F20", 0x00100000814),
    ("F21", 0x00100000815),
    ("F22", 0x00100000816),
    ("F23", 0x00100000817),
    ("F24", 0x00100000818),
    ("F3", 0x00100000803),
    ("F4", 0x00100000804),
    ("F5", 0x00100000805),
    ("F6", 0x00100000806),
    ("F7", 0x00100000807),
    ("F8", 0x00100000808),
    ("F9", 0x00100000809),
    ("FavoriteClear0", 0x00100000d16),
    ("FavoriteClear1", 0x00100000d17),
    ("FavoriteClear2", 0x00100000d18),
    ("FavoriteClear3", 0x00100000d19),
    ("FavoriteRecall0", 0x00100000d1a),
    ("FavoriteRecall1", 0x00100000d1b),
    ("FavoriteRecall2", 0x00100000d1c),
    ("FavoriteRecall3", 0x00100000d1d),
    ("FavoriteStore0", 0x00100000d1e),
    ("FavoriteStore1", 0x00100000d1f),
    ("FavoriteStore2", 0x00100000d20),
    ("FavoriteStore3", 0x00100000d21),
    ("FinalMode", 0x00100000706),
    ("Find", 0x00100000507),
    ("Fn", 0x00100000106),
    ("FnLock", 0x00100000107),
    ("GoBack", 0x00100001005),
    ("GoHome", 0x00100001006),
    ("GroupFirst", 0x00100000707),
    ("GroupLast", 0x00100000708),
    ("GroupNext", 0x00100000709),
    ("GroupPrevious", 0x0010000070a),
    ("Guide", 0x00100000d22),
    ("GuideNextDay", 0x00100000d23),
    ("GuidePreviousDay", 0x00100000d24),
    ("HangulMode", 0x00100000711),
    ("HanjaMode", 0x00100000712),
    ("Hankaku", 0x00100000715),
    ("HeadsetHook", 0x00100001007),
    ("Help", 0x00100000508),
    ("Hibernate", 0x00100000609),
    ("Hiragana", 0x00100000716),
    ("HiraganaKatakana", 0x00100000717),
    ("Home", 0x00100000306),
    ("Hyper", 0x00100000108),
    ("Info", 0x00100000d25),
    ("Insert", 0x00100000407),
    ("InstantReplay", 0x00100000d26),
    ("JunjaMode", 0x00100000713),
    ("KanaMode", 0x00100000718),
    ("KanjiMode", 0x00100000719),
    ("Katakana", 0x0010000071a),
    ("Key11", 0x00100001201),
    ("Key12", 0x00100001202),
    ("LastNumberRedial", 0x00100001008),
    ("LaunchApplication1", 0x00100000b06),
    ("LaunchApplication2", 0x00100000b01),
    ("LaunchAssistant", 0x00100000b0e),
    ("LaunchCalendar", 0x00100000b02),
    ("LaunchContacts", 0x00100000b0c),
    ("LaunchControlPanel", 0x00100000b0f),
    ("LaunchMail", 0x00100000b03),
    ("LaunchMediaPlayer", 0x00100000b04),
    ("LaunchMusicPlayer", 0x00100000b05),
    ("LaunchPhone", 0x00100000b0d),
    ("LaunchScreenSaver", 0x00100000b07),
    ("LaunchSpreadsheet", 0x00100000b08),
    ("LaunchWebBrowser", 0x00100000b09),
    ("LaunchWebCam", 0x00100000b0a),
    ("LaunchWordProcessor", 0x00100000b0b),
    ("Link", 0x00100000d27),
    ("ListProgram", 0x00100000d28),
    ("LiveContent", 0x00100000d29),
    ("Lock", 0x00100000d2a),
    ("LogOff", 0x00100000605),
    ("MailForward", 0x00100000a02),
    ("MailReply", 0x00100000a03),
    ("MailSend", 0x00100000a04),
    ("MannerMode", 0x0010000100a),
    ("MediaApps", 0x00100000d2b),
    ("MediaAudioTrack", 0x00100000d50),
    ("MediaClose", 0x00100000d5b),
    ("MediaFastForward", 0x00100000d2c),
    ("MediaLast", 0x00100000d2d),
    ("MediaPause", 0x00100000d2e),
    ("MediaPlay", 0x00100000d2f),
    ("MediaPlayPause", 0x00100000a05),
    ("MediaRecord", 0x00100000d30),
    ("MediaRewind", 0x00100000d31),
    ("MediaSkip", 0x00100000d32),
    ("MediaSkipBackward", 0x00100000d51),
    ("MediaSkipForward", 0x00100000d52),
    ("MediaStepBackward", 0x00100000d53),
    ("MediaStepForward", 0x00100000d54),
    ("MediaStop", 0x00100000a07),
    ("MediaTopMenu", 0x00100000d55),
    ("MediaTrackNext", 0x00100000a08),
    ("MediaTrackPrevious", 0x00100000a09),
    ("MicrophoneToggle", 0x00100000e06),
    ("MicrophoneVolumeDown", 0x00100000e07),
    ("MicrophoneVolumeMute", 0x00100000e09),
    ("MicrophoneVolumeUp", 0x00100000e08),
    ("ModeChange", 0x0010000070b),
    ("NavigateIn", 0x00100000d56),
    ("NavigateNext", 0x00100000d57),
    ("NavigateOut", 0x00100000d58),
    ("NavigatePrevious", 0x00100000d59),
    ("New", 0x00100000a0a),
    ("NextCandidate", 0x0010000070c),
    ("NextFavoriteChannel", 0x00100000d33),
    ("NextUserProfile", 0x00100000d34),
    ("NonConvert", 0x0010000070d),
    ("Notification", 0x00100001009),
    ("NumLock", 0x0010000010a),
    ("OnDemand", 0x00100000d35),
    ("Open", 0x00100000a0b),
    ("PageDown", 0x00100000307),
    ("PageUp", 0x00100000308),
    ("Pairing", 0x00100000d5a),
    ("Paste", 0x00100000408),
    ("Pause", 0x00100000509),
    ("PinPDown", 0x00100000d36),
    ("PinPMove", 0x00100000d37),
    ("PinPToggle", 0x00100000d38),
    ("PinPUp", 0x00100000d39),
    ("Play", 0x0010000050a),
    ("PlaySpeedDown", 0x00100000d3a),
    ("PlaySpeedReset", 0x00100000d3b),
    ("PlaySpeedUp", 0x00100000d3c),
    ("Power", 0x00100000606),
    ("PowerOff", 0x00100000607),
    ("PreviousCandidate", 0x0010000070e),
    ("Print", 0x00100000a0c),
    ("PrintScreen", 0x00100000608),
    ("Process", 0x0010000070f),
    ("Props", 0x0010000050b),
    ("RandomToggle", 0x00100000d3d),
    ("RcLowBattery", 0x00100000d3e),
    ("RecordSpeedNext", 0x00100000d3f),
    ("Redo", 0x00100000409),
    ("RfBypass", 0x00100000d40),
    ("Romaji", 0x0010000071b),
    ("STBInput", 0x00100000d45),
    ("STBPower", 0x00100000d46),
    ("Save", 0x00100000a0d),
    ("ScanChannelsToggle", 0x00100000d41),
    ("ScreenModeNext", 0x00100000d42),
    ("ScrollLock", 0x0010000010c),
    ("Select", 0x0010000050c),
    ("Settings", 0x00100000d43),
    ("ShiftLevel5", 0x00100000111),
    ("SingleCandidate", 0x00100000710),
    ("Soft1", 0x00100000901),
    ("Soft2", 0x00100000902),
    ("Soft3", 0x00100000903),
    ("Soft4", 0x00100000904),
    ("Soft5", 0x00100000905),
    ("Soft6", 0x00100000906),
    ("Soft7", 0x00100000907),
    ("Soft8", 0x00100000908),
    ("SpeechCorrectionList", 0x00100000f01),
    ("SpeechInputToggle", 0x00100000f02),
    ("SpellCheck", 0x00100000a0e),
    ("SplitScreenToggle", 0x00100000d44),
    ("Standby", 0x0010000060a),
    ("Subtitle", 0x00100000d47),
    ("Super", 0x0010000010e),
    ("Symbol", 0x0010000010f),
    ("SymbolLock", 0x00100000110),
    ("TV", 0x00100000d49),
    ("TV3DMode", 0x00100001101),
    ("TVAntennaCable", 0x00100001102),
    ("TVAudioDescription", 0x00100001103),
    ("TVAudioDescriptionMixDown", 0x00100001104),
    ("TVAudioDescriptionMixUp", 0x00100001105),
    ("TVContentsMenu", 0x00100001106),
    ("TVDataService", 0x00100001107),
    ("TVInput", 0x00100000d4a),
    ("TVInputComponent1", 0x00100001108),
    ("TVInputComponent2", 0x00100001109),
    ("TVInputComposite1", 0x0010000110a),
    ("TVInputComposite2", 0x0010000110b),
    ("TVInputHDMI1", 0x0010000110c),
    ("TVInputHDMI2", 0x0010000110d),
    ("TVInputHDMI3", 0x0010000110e),
    ("TVInputHDMI4", 0x0010000110f),
    ("TVInputVGA1", 0x00100001110),
    ("TVMediaContext", 0x00100001111),
    ("TVNetwork", 0x00100001112),
    ("TVNumberEntry", 0x00100001113),
    ("TVPower", 0x00100000d4b),
    ("TVRadioService", 0x00100001114),
    ("TVSatellite", 0x00100001115),
    ("TVSatelliteBS", 0x00100001116),
    ("TVSatelliteCS", 0x00100001117),
    ("TVSatelliteToggle", 0x00100001118),
    ("TVTerrestrialAnalog", 0x00100001119),
    ("TVTerrestrialDigital", 0x0010000111a),
    ("TVTimer", 0x0010000111b),
    ("Tab", 0x00100000009),
    ("Teletext", 0x00100000d48),
    ("Undo", 0x0010000040a),
    ("Unidentified", 0x00100000001),
    ("VideoModeNext", 0x00100000d4c),
    ("VoiceDial", 0x0010000100b),
    ("WakeUp", 0x0010000060b),
    ("Wink", 0x00100000d4d),
    ("Zenkaku", 0x0010000071c),
    ("ZenkakuHankaku", 0x0010000071d),
    ("ZoomIn", 0x0010000050d),
    ("ZoomOut", 0x0010000050e),
    ("ZoomToggle", 0x00100000d4e),
];

static LOGICAL_LOCATION: &[(&str, [Option<u64>; 4])] = &[
    ("*", [Some(0x0000000002a), None, None, Some(0x0020000022a)]),
    ("+", [Some(0x0000000002b), None, None, Some(0x0020000022b)]),
    ("-", [Some(0x0000000002d), None, None, Some(0x0020000022d)]),
    (".", [Some(0x0000000002e), None, None, Some(0x0020000022e)]),
    ("/", [Some(0x0000000002f), None, None, Some(0x0020000022f)]),
    ("0", [Some(0x00000000030), None, None, Some(0x00200000230)]),
    ("1", [Some(0x00000000031), None, None, Some(0x00200000231)]),
    ("2", [Some(0x00000000032), None, None, Some(0x00200000232)]),
    ("3", [Some(0x00000000033), None, None, Some(0x00200000233)]),
    ("4", [Some(0x00000000034), None, None, Some(0x00200000234)]),
    ("5", [Some(0x00000000035), None, None, Some(0x00200000235)]),
    ("6", [Some(0x00000000036), None, None, Some(0x00200000236)]),
    ("7", [Some(0x00000000037), None, None, Some(0x00200000237)]),
    ("8", [Some(0x00000000038), None, None, Some(0x00200000238)]),
    ("9", [Some(0x00000000039), None, None, Some(0x00200000239)]),
    (
        "Alt",
        [
            Some(0x00200000104),
            Some(0x00200000104),
            Some(0x00200000105),
            None,
        ],
    ),
    (
        "AltGraph",
        [Some(0x00100000103), None, Some(0x00100000103), None],
    ),
    (
        "ArrowDown",
        [Some(0x00100000301), None, None, Some(0x00200000232)],
    ),
    (
        "ArrowLeft",
        [Some(0x00100000302), None, None, Some(0x00200000234)],
    ),
    (
        "ArrowRight",
        [Some(0x00100000303), None, None, Some(0x00200000236)],
    ),
    (
        "ArrowUp",
        [Some(0x00100000304), None, None, Some(0x00200000238)],
    ),
    (
        "Clear",
        [Some(0x00100000401), None, None, Some(0x00200000235)],
    ),
    (
        "Control",
        [
            Some(0x00200000100),
            Some(0x00200000100),
            Some(0x00200000101),
            None,
        ],
    ),
    (
        "Delete",
        [Some(0x0010000007f), None, None, Some(0x0020000022e)],
    ),
    (
        "End",
        [Some(0x00100000305), None, None, Some(0x00200000231)],
    ),
    (
        "Enter",
        [Some(0x0010000000d), None, None, Some(0x0020000020d)],
    ),
    (
        "Home",
        [Some(0x00100000306), None, None, Some(0x00200000237)],
    ),
    (
        "Insert",
        [Some(0x00100000407), None, None, Some(0x00200000230)],
    ),
    (
        "Meta",
        [
            Some(0x00200000106),
            Some(0x00200000106),
            Some(0x00200000107),
            None,
        ],
    ),
    (
        "PageDown",
        [Some(0x00100000307), None, None, Some(0x00200000233)],
    ),
    (
        "PageUp",
        [Some(0x00100000308), None, None, Some(0x00200000239)],
    ),
    (
        "Shift",
        [
            Some(0x00200000102),
            Some(0x00200000102),
            Some(0x00200000103),
            None,
        ],
    ),
];

#[cfg(test)]
mod tests {
    use winit::keyboard::{Key, KeyCode, KeyLocation, NamedKey, PhysicalKey};

    use super::{character_of, logical_key_id, physical_key_usage};

    fn character(text: &str) -> Key {
        Key::Character(text.into())
    }

    #[test]
    fn letters_and_digits_map_to_their_usb_usage_and_lower_case_code_point() {
        assert_eq!(
            physical_key_usage(PhysicalKey::Code(KeyCode::KeyA)),
            Some(0x00070004)
        );
        assert_eq!(
            physical_key_usage(PhysicalKey::Code(KeyCode::Digit7)),
            Some(0x00070024)
        );
        assert_eq!(
            logical_key_id(&character("a"), KeyLocation::Standard),
            Some(0x00000061)
        );
        assert_eq!(
            logical_key_id(&character("A"), KeyLocation::Standard),
            Some(0x00000061),
            "a shifted letter is the same logical key"
        );
        assert_eq!(
            logical_key_id(&character("7"), KeyLocation::Standard),
            Some(0x00000037)
        );
    }

    #[test]
    fn named_keys_map_to_the_unprintable_plane() {
        assert_eq!(
            physical_key_usage(PhysicalKey::Code(KeyCode::Enter)),
            Some(0x00070028)
        );
        assert_eq!(
            logical_key_id(&Key::Named(NamedKey::Enter), KeyLocation::Standard),
            Some(0x0010000000d)
        );
        assert_eq!(
            physical_key_usage(PhysicalKey::Code(KeyCode::Escape)),
            Some(0x00070029)
        );
        assert_eq!(
            logical_key_id(&Key::Named(NamedKey::Escape), KeyLocation::Standard),
            Some(0x0010000001b)
        );
        assert_eq!(
            physical_key_usage(PhysicalKey::Code(KeyCode::ArrowLeft)),
            Some(0x00070050)
        );
        assert_eq!(
            logical_key_id(&Key::Named(NamedKey::ArrowLeft), KeyLocation::Standard),
            Some(0x00100000302)
        );
        assert_eq!(
            logical_key_id(&Key::Named(NamedKey::Space), KeyLocation::Standard),
            Some(0x00000020),
            "the space bar produces a character, not a name"
        );
    }

    #[test]
    fn a_modifier_takes_its_side_from_the_key_location() {
        assert_eq!(
            physical_key_usage(PhysicalKey::Code(KeyCode::ShiftLeft)),
            Some(0x000700e1)
        );
        assert_eq!(
            physical_key_usage(PhysicalKey::Code(KeyCode::ShiftRight)),
            Some(0x000700e5)
        );
        assert_eq!(
            logical_key_id(&Key::Named(NamedKey::Shift), KeyLocation::Left),
            Some(0x00200000102)
        );
        assert_eq!(
            logical_key_id(&Key::Named(NamedKey::Shift), KeyLocation::Right),
            Some(0x00200000103)
        );
        assert_eq!(
            logical_key_id(&Key::Named(NamedKey::Control), KeyLocation::Right),
            Some(0x00200000101)
        );
        assert_eq!(
            logical_key_id(&Key::Named(NamedKey::Shift), KeyLocation::Standard),
            Some(0x00200000102),
            "a side-less report is the left key, as on the web"
        );
    }

    #[test]
    fn a_numpad_digit_is_its_own_logical_key() {
        assert_eq!(
            physical_key_usage(PhysicalKey::Code(KeyCode::Numpad5)),
            Some(0x0007005d)
        );
        assert_eq!(
            logical_key_id(&character("5"), KeyLocation::Numpad),
            Some(0x00200000235)
        );
        assert_eq!(
            logical_key_id(&character("5"), KeyLocation::Standard),
            Some(0x00000035)
        );
    }

    #[test]
    fn winit_keys_flutter_has_no_code_for_are_unmapped() {
        assert_eq!(physical_key_usage(PhysicalKey::Code(KeyCode::F25)), None);
        assert_eq!(
            logical_key_id(&Key::Named(NamedKey::F25), KeyLocation::Standard),
            None
        );
        assert_eq!(
            logical_key_id(&Key::Dead(Some('^')), KeyLocation::Standard),
            None
        );
    }

    #[test]
    fn control_characters_are_not_reported_as_input() {
        assert_eq!(character_of(Some("a")), Some("a".to_owned()));
        assert_eq!(character_of(Some(" ")), Some(" ".to_owned()));
        assert_eq!(character_of(Some("\r")), None);
        assert_eq!(character_of(Some("\u{1b}")), None);
        assert_eq!(character_of(None), None);
    }
}
