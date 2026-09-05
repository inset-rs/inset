//! Flutter counterpart: `cupertino/icons.dart`.
//!
//! Generated from the Dart table; the `<i class='cupertino-icons'>` glyph previews and the
//! `{@tool}` macros are dropped. The font itself is the `cupertino_icons` 1.0.9 asset,
//! bundled under `assets/` and registered by [`install_cupertino_icon_font`].

use reveal_foundation::{App, Handle};
use reveal_painting::PaintingBinding;
use reveal_widgets::IconData;

/// The `cupertino_icons` font asset, which Flutter bundles through the package's
/// `pubspec.yaml` and this crate carries in `assets/` (see `assets/LICENSE`).
const ICON_FONT_BYTES: &[u8] = include_bytes!("../assets/CupertinoIcons.ttf");

/// Identifiers for the supported Cupertino icons.
///
/// Use with the `Icon` class to show specific icons.
///
/// Icons are identified by their name as listed below.
///
/// Where a Flutter app depends on the `cupertino_icons` package for the font, this crate
/// bundles it: call [`install_cupertino_icon_font`] once at start-up.
///
/// For versions 0.1.3 and below, see this [glyph map](https://raw.githubusercontent.com/flutter/packages/main/third_party/packages/cupertino_icons/map.png).
///
/// See also:
///
///  * `Icon`, used to show these icons.
pub struct CupertinoIcons;

/// An icon of the bundled font: every entry of the table carries the family and the package
/// the glyph lives in.
fn cupertino(code_point: u32) -> IconData {
    IconData::new(code_point)
        .font_family(CupertinoIcons::ICON_FONT)
        .font_package(CupertinoIcons::ICON_FONT_PACKAGE)
}

/// The family the icon font is registered under.
///
/// Flutter's asset tooling prefixes a family a package declares with `packages/<package>/`,
/// and that is the name `TextStyle::package` asks for.
fn icon_font_family() -> String {
    format!(
        "packages/{}/{}",
        CupertinoIcons::ICON_FONT_PACKAGE,
        CupertinoIcons::ICON_FONT
    )
}

/// Whether the app-wide font collection already carries the icon font.
fn is_icon_font_installed(binding: Handle<PaintingBinding>, app: &App, family: &str) -> bool {
    binding.has_fonts(app) && app.get(binding.fonts(app)).family(family).is_some()
}

/// Registers the bundled Cupertino icons font with the app-wide font collection, which is
/// what the `cupertino_icons` font asset declaration in `pubspec.yaml` does in Flutter.
///
/// Call it once at start-up, before a tree that shows an icon is built. Calling it again
/// does nothing.
pub fn install_cupertino_icon_font(app: &mut App) {
    let binding = PaintingBinding::instance(app);
    let family = icon_font_family();
    if is_icon_font_installed(binding, app, &family) {
        return;
    }
    binding
        .register_font(app, &family, ICON_FONT_BYTES.to_vec())
        .expect("the bundled Cupertino icons font parses");
}

impl CupertinoIcons {
    /// The icon font used for Cupertino icons.
    pub const ICON_FONT: &str = "CupertinoIcons";

    /// The dependent package providing the Cupertino icons font.
    pub const ICON_FONT_PACKAGE: &str = "cupertino_icons";

    // ===========================================================================
    // BEGIN LEGACY PRE SF SYMBOLS NAMES
    // We need to leave them as-is with the same codepoints for backward
    // compatibility with cupertino_icons <0.1.3.

    /// Cupertino icon for a thin left chevron.
    /// This is the same icon as [`chevron_left`](Self::chevron_left()) in cupertino_icons 1.0.0+.
    pub fn left_chevron() -> IconData {
        cupertino(0xf3d2).match_text_direction(true)
    }

    /// Cupertino icon for a thin right chevron.
    /// This is the same icon as [`chevron_right`](Self::chevron_right()) in cupertino_icons 1.0.0+.
    pub fn right_chevron() -> IconData {
        cupertino(0xf3d3).match_text_direction(true)
    }

    /// Cupertino icon for an iOS style share icon with an arrow pointing up from a box. This icon is not filled in.
    /// This is the same icon as [`square_arrow_up`](Self::square_arrow_up()) and [`share_up`](Self::share_up()) in cupertino_icons 1.0.0+.
    ///
    /// See also:
    ///
    ///  * [`share_solid`](Self::share_solid()), which is similar, but filled in.
    ///  * [`share_up`](Self::share_up()), for another (pre-iOS 7) version of this icon.
    pub fn share() -> IconData {
        cupertino(0xf4ca)
    }

    /// Cupertino icon for an iOS style share icon with an arrow pointing up from a box. This icon is filled in.
    /// This is the same icon as [`square_arrow_up_fill`](Self::square_arrow_up_fill()) in cupertino_icons 1.0.0+.
    ///
    /// See also:
    ///
    ///  * [`share`](Self::share()), which is similar, but not filled in.
    ///  * [`share_up`](Self::share_up()), for another (pre-iOS 7) version of this icon.
    pub fn share_solid() -> IconData {
        cupertino(0xf4cb)
    }

    /// Cupertino icon for a book silhouette spread open. This icon is not filled in.
    /// See also:
    ///
    ///  * [`book_solid`](Self::book_solid()), which is similar, but filled in.
    pub fn book() -> IconData {
        cupertino(0xf3e7)
    }

    /// Cupertino icon for a book silhouette spread open. This icon is filled in.
    /// This is the same icon as [`book_fill`](Self::book_fill()) in cupertino_icons 1.0.0+.
    ///
    /// See also:
    ///
    ///  * [`book`](Self::book()), which is similar, but not filled in.
    pub fn book_solid() -> IconData {
        cupertino(0xf3e8)
    }

    /// Cupertino icon for a book silhouette spread open containing a bookmark in the upper right. This icon is not filled in.
    ///
    /// See also:
    ///
    ///  * [`bookmark_solid`](Self::bookmark_solid()), which is similar, but filled in.
    pub fn bookmark() -> IconData {
        cupertino(0xf3e9)
    }

    /// Cupertino icon for a book silhouette spread open containing a bookmark in the upper right. This icon is filled in.
    /// This is the same icon as [`bookmark_fill`](Self::bookmark_fill()) in cupertino_icons 1.0.0+.
    ///
    /// See also:
    ///
    ///  * [`bookmark`](Self::bookmark()), which is similar, but not filled in.
    pub fn bookmark_solid() -> IconData {
        cupertino(0xf3ea)
    }

    /// Cupertino icon for a letter 'i' in a circle.
    /// This is the same icon as [`info_circle`](Self::info_circle()) in cupertino_icons 1.0.0+.
    pub fn info() -> IconData {
        cupertino(0xf44c)
    }

    /// Cupertino icon for a curved up and left pointing arrow.
    /// This is the same icon as [`arrowshape_turn_up_left`](Self::arrowshape_turn_up_left()) in cupertino_icons 1.0.0+.
    ///
    /// For another version of this icon, see [`reply_thick_solid`](Self::reply_thick_solid()).
    pub fn reply() -> IconData {
        cupertino(0xf4c6)
    }

    /// Cupertino icon for a chat bubble.
    /// This is the same icon as [`chat_bubble`](Self::chat_bubble()) in cupertino_icons 1.0.0+.
    pub fn conversation_bubble() -> IconData {
        cupertino(0xf3fb)
    }

    /// Cupertino icon for a person's silhouette in a circle.
    /// This is the same icon as [`person_crop_circle`](Self::person_crop_circle()) in cupertino_icons 1.0.0+.
    pub fn profile_circled() -> IconData {
        cupertino(0xf419)
    }

    /// Cupertino icon for a '+' sign in a circle.
    /// This is the same icon as [`plus_circle`](Self::plus_circle()) in cupertino_icons 1.0.0+.
    pub fn plus_circled() -> IconData {
        cupertino(0xf48a)
    }

    /// Cupertino icon for a '-' sign in a circle.
    /// This is the same icon as [`minus_circle`](Self::minus_circle()) in cupertino_icons 1.0.0+.
    pub fn minus_circled() -> IconData {
        cupertino(0xf463)
    }

    /// Cupertino icon for a right facing flag and pole outline.
    pub fn flag() -> IconData {
        cupertino(0xf42c)
    }

    /// Cupertino icon for a magnifier loop outline.
    pub fn search() -> IconData {
        cupertino(0xf4a5)
    }

    /// Cupertino icon for a checkmark.
    /// This is the same icon as [`checkmark`](Self::checkmark()) in cupertino_icons 1.0.0+.
    ///
    /// See also:
    ///
    ///  * [`check_mark_circled`](Self::check_mark_circled()), which consists of this check mark and a circle surrounding it.
    pub fn check_mark() -> IconData {
        cupertino(0xf3fd)
    }

    /// Cupertino icon for a checkmark in a circle. The circle is not filled in.
    /// This is the same icon as [`checkmark_circle`](Self::checkmark_circle()) in cupertino_icons 1.0.0+.
    ///
    /// See also:
    ///
    ///  * [`check_mark_circled_solid`](Self::check_mark_circled_solid()), which is similar, but filled in.
    ///  * [`check_mark`](Self::check_mark()), which is the check mark without a circle.
    pub fn check_mark_circled() -> IconData {
        cupertino(0xf3fe)
    }

    /// Cupertino icon for a checkmark in a circle. The circle is filled in.
    /// This is the same icon as [`checkmark_circle_fill`](Self::checkmark_circle_fill()) in cupertino_icons 1.0.0+.
    ///
    /// See also:
    ///
    ///  * [`check_mark_circled`](Self::check_mark_circled()), which is similar, but not filled in.
    pub fn check_mark_circled_solid() -> IconData {
        cupertino(0xf3ff)
    }

    /// Cupertino icon for an empty circle (a ring). An un-selected radio button.
    ///
    /// See also:
    ///
    ///  * [`circle_filled`](Self::circle_filled()), which is similar but filled in.
    pub fn circle() -> IconData {
        cupertino(0xf401)
    }

    /// Cupertino icon for a filled circle. The circle is surrounded by a ring. A selected radio button.
    /// This is the same icon as [`circle_fill`](Self::circle_fill()) in cupertino_icons 1.0.0+.
    ///
    /// See also:
    ///
    ///  * [`circle`](Self::circle()), which is similar but not filled in.
    pub fn circle_filled() -> IconData {
        cupertino(0xf400)
    }

    /// Cupertino icon for a thicker left chevron used in iOS for the navigation bar back button.
    /// This is the same icon as [`chevron_back`](Self::chevron_back()) in cupertino_icons 1.0.0+.
    pub fn back() -> IconData {
        cupertino(0xf3cf).match_text_direction(true)
    }

    /// Cupertino icon for a thicker right chevron that's the reverse of [`back`](Self::back()).
    /// This is the same icon as [`chevron_forward`](Self::chevron_forward()) in cupertino_icons 1.0.0+.
    pub fn forward() -> IconData {
        cupertino(0xf3d1).match_text_direction(true)
    }

    /// Cupertino icon for an outline of a simple front-facing house.
    /// This is the same icon as [`house`](Self::house()) in cupertino_icons 1.0.0+.
    pub fn home() -> IconData {
        cupertino(0xf447)
    }

    /// Cupertino icon for a right-facing shopping cart outline.
    /// This is the same icon as [`cart`](Self::cart()) in cupertino_icons 1.0.0+.
    pub fn shopping_cart() -> IconData {
        cupertino(0xf3f7)
    }

    /// Cupertino icon for three solid dots.
    pub fn ellipsis() -> IconData {
        cupertino(0xf46a)
    }

    /// Cupertino icon for a phone handset outline.
    pub fn phone() -> IconData {
        cupertino(0xf4b8)
    }

    /// Cupertino icon for a phone handset.
    /// This is the same icon as [`phone_fill`](Self::phone_fill()) in cupertino_icons 1.0.0+.
    pub fn phone_solid() -> IconData {
        cupertino(0xf4b9)
    }

    /// Cupertino icon for a solid down arrow.
    /// This is the same icon as [`arrow_down`](Self::arrow_down()) in cupertino_icons 1.0.0+.
    pub fn down_arrow() -> IconData {
        cupertino(0xf35d)
    }

    /// Cupertino icon for a solid up arrow.
    /// This is the same icon as [`arrow_up`](Self::arrow_up()) in cupertino_icons 1.0.0+.
    pub fn up_arrow() -> IconData {
        cupertino(0xf366)
    }

    /// Cupertino icon for a charging battery.
    /// This is the same icon as [`battery_100`](Self::battery_100()), [`battery_full`](Self::battery_full()) and [`battery_75_percent`](Self::battery_75_percent()) in cupertino_icons 1.0.0+.
    pub fn battery_charging() -> IconData {
        cupertino(0xf111)
    }

    /// Cupertino icon for an empty battery.
    /// This is the same icon as [`battery_0`](Self::battery_0()) in cupertino_icons 1.0.0+.
    pub fn battery_empty() -> IconData {
        cupertino(0xf112)
    }

    /// Cupertino icon for a full battery.
    /// This is the same icon as [`battery_100`](Self::battery_100()), [`battery_charging`](Self::battery_charging()) and [`battery_75_percent`](Self::battery_75_percent()) in cupertino_icons 1.0.0+.
    pub fn battery_full() -> IconData {
        cupertino(0xf113)
    }

    /// Cupertino icon for a 75% charged battery.
    /// This is the same icon as [`battery_100`](Self::battery_100()), [`battery_charging`](Self::battery_charging()) and [`battery_full`](Self::battery_full()) in cupertino_icons 1.0.0+.
    pub fn battery_75_percent() -> IconData {
        cupertino(0xf114)
    }

    /// Cupertino icon for a 25% charged battery.
    /// This is the same icon as [`battery_25`](Self::battery_25()) in cupertino_icons 1.0.0+.
    pub fn battery_25_percent() -> IconData {
        cupertino(0xf115)
    }

    /// Cupertino icon for the Bluetooth logo.
    /// This icon is available in cupertino_icons 1.0.0+ for backward
    /// compatibility but not part of Apple icons' aesthetics.
    pub fn bluetooth() -> IconData {
        cupertino(0xf116)
    }

    /// Cupertino icon for a restart arrow, pointing downwards.
    /// This is the same icon as [`arrow_counterclockwise`](Self::arrow_counterclockwise()) in cupertino_icons 1.0.0+.
    pub fn restart() -> IconData {
        cupertino(0xf21c)
    }

    /// Cupertino icon for two curved up and left pointing arrows.
    /// This is the same icon as [`arrowshape_turn_up_left_2`](Self::arrowshape_turn_up_left_2()) in cupertino_icons 1.0.0+.
    pub fn reply_all() -> IconData {
        cupertino(0xf21d)
    }

    /// Cupertino icon for a curved up and left pointing arrow.
    /// This is the same icon as [`arrowshape_turn_up_left_2_fill`](Self::arrowshape_turn_up_left_2_fill()) in cupertino_icons 1.0.0+.
    ///
    /// For another version of this icon, see [`reply`](Self::reply()).
    pub fn reply_thick_solid() -> IconData {
        cupertino(0xf21e)
    }

    /// Cupertino icon for an iOS style share icon with an arrow pointing upwards to the right from a box.
    /// This is the same icon as [`square_arrow_up`](Self::square_arrow_up()) and [`share_up`](Self::share_up()) in cupertino_icons 1.0.0+.
    ///
    /// For another version of this icon (introduced in iOS 7), see [`share`](Self::share()).
    pub fn share_up() -> IconData {
        cupertino(0xf220)
    }

    /// Cupertino icon for two thin right-facing intertwined arrows.
    /// This is the same icon as [`shuffle_medium`](Self::shuffle_medium()) and [`shuffle_thick`](Self::shuffle_thick()) in cupertino_icons 1.0.0+.
    ///
    /// See also:
    ///
    ///  * [`shuffle_medium`](Self::shuffle_medium()), with slightly thicker arrows.
    ///  * [`shuffle_thick`](Self::shuffle_thick()), with thicker, bold arrows.
    pub fn shuffle() -> IconData {
        cupertino(0xf4a9)
    }

    /// Cupertino icon for an two medium thickness right-facing intertwined arrows.
    /// This is the same icon as [`shuffle`](Self::shuffle()) and [`shuffle_thick`](Self::shuffle_thick()) in cupertino_icons 1.0.0+.
    ///
    /// See also:
    ///
    ///  * [`shuffle`](Self::shuffle()), with thin arrows.
    ///  * [`shuffle_thick`](Self::shuffle_thick()), with thicker, bold arrows.
    pub fn shuffle_medium() -> IconData {
        cupertino(0xf4a8)
    }

    /// Cupertino icon for two thick right-facing intertwined arrows.
    /// This is the same icon as [`shuffle_medium`](Self::shuffle_medium()) and [`shuffle`](Self::shuffle()) in cupertino_icons 1.0.0+.
    ///
    /// See also:
    ///
    ///  * [`shuffle`](Self::shuffle()), with thin arrows.
    ///  * [`shuffle_medium`](Self::shuffle_medium()), with slightly thinner arrows.
    pub fn shuffle_thick() -> IconData {
        cupertino(0xf221)
    }

    /// Cupertino icon for a camera for still photographs. This icon is filled in.
    /// This is the same icon as [`camera`](Self::camera()) in cupertino_icons 1.0.0+.
    ///
    /// See also:
    ///
    ///  * [`photo_camera`](Self::photo_camera()), which is similar, but not filled in.
    ///  * [`video_camera_solid`](Self::video_camera_solid()), for the moving picture equivalent.
    pub fn photo_camera() -> IconData {
        cupertino(0xf3f5)
    }

    /// Cupertino icon for a camera for still photographs. This icon is not filled in.
    /// This is the same icon as [`camera_fill`](Self::camera_fill()) in cupertino_icons 1.0.0+.
    ///
    /// See also:
    ///
    ///  * [`photo_camera_solid`](Self::photo_camera_solid()), which is similar, but filled in.
    ///  * [`video_camera`](Self::video_camera()), for the moving picture equivalent.
    pub fn photo_camera_solid() -> IconData {
        cupertino(0xf3f6)
    }

    /// Cupertino icon for a camera for moving pictures. This icon is not filled in.
    /// This is the same icon as [`videocam`](Self::videocam()) in cupertino_icons 1.0.0+.
    ///
    /// See also:
    ///
    ///  * [`video_camera_solid`](Self::video_camera_solid()), which is similar, but filled in.
    ///  * [`photo_camera`](Self::photo_camera()), for the still photograph equivalent.
    pub fn video_camera() -> IconData {
        cupertino(0xf4cc)
    }

    /// Cupertino icon for a camera for moving pictures. This icon is filled in.
    /// This is the same icon as [`videocam_fill`](Self::videocam_fill()) in cupertino_icons 1.0.0+.
    ///
    /// See also:
    ///
    ///  * [`video_camera`](Self::video_camera()), which is similar, but not filled in.
    ///  * [`photo_camera_solid`](Self::photo_camera_solid()), for the still photograph equivalent.
    pub fn video_camera_solid() -> IconData {
        cupertino(0xf4cd)
    }

    /// Cupertino icon for a camera containing two circular arrows pointing at each other, which indicate switching. This icon is not filled in.
    /// This is the same icon as [`camera_rotate`](Self::camera_rotate()) in cupertino_icons 1.0.0+.
    ///
    /// See also:
    ///
    ///  * [`switch_camera_solid`](Self::switch_camera_solid()), which is similar, but filled in.
    pub fn switch_camera() -> IconData {
        cupertino(0xf49e)
    }

    /// Cupertino icon for a camera containing two circular arrows pointing at each other, which indicate switching. This icon is filled in.
    /// This is the same icon as [`camera_rotate_fill`](Self::camera_rotate_fill()) in cupertino_icons 1.0.0+.
    ///
    /// See also:
    ///
    ///  * [`switch_camera`](Self::switch_camera()), which is similar, but not filled in.
    pub fn switch_camera_solid() -> IconData {
        cupertino(0xf49f)
    }

    /// Cupertino icon for a collection of folders, which store collections of files, i.e. an album. This icon is not filled in.
    /// This is the same icon as [`rectangle_stack`](Self::rectangle_stack()) in cupertino_icons 1.0.0+.
    ///
    /// See also:
    ///
    ///  * [`collections_solid`](Self::collections_solid()), which is similar, but filled in.
    pub fn collections() -> IconData {
        cupertino(0xf3c9)
    }

    /// Cupertino icon for a collection of folders, which store collections of files, i.e. an album. This icon is filled in.
    /// This is the same icon as [`rectangle_stack_fill`](Self::rectangle_stack_fill()) in cupertino_icons 1.0.0+.
    ///
    /// See also:
    ///
    ///  * [`collections`](Self::collections()), which is similar, but not filled in.
    pub fn collections_solid() -> IconData {
        cupertino(0xf3ca)
    }

    /// Cupertino icon for a single folder, which stores multiple files. This icon is not filled in.
    /// This is the same icon as [`folder_open`](Self::folder_open()) in cupertino_icons 1.0.0+.
    ///
    /// See also:
    ///
    ///  * [`folder_solid`](Self::folder_solid()), which is similar, but filled in.
    ///  * [`folder_open`](Self::folder_open()), which is the pre-iOS 7 version of this icon.
    pub fn folder() -> IconData {
        cupertino(0xf434)
    }

    /// Cupertino icon for a single folder, which stores multiple files. This icon is filled in.
    /// This is the same icon as [`folder_fill`](Self::folder_fill()) in cupertino_icons 1.0.0+.
    ///
    /// See also:
    ///
    ///  * [`folder`](Self::folder()), which is similar, but not filled in.
    ///  * [`folder_open`](Self::folder_open()), which is the pre-iOS 7 version of this icon and not filled in.
    pub fn folder_solid() -> IconData {
        cupertino(0xf435)
    }

    /// Cupertino icon for a single folder that indicates being opened. A folder like this typically stores multiple files.
    /// This is the same icon as [`folder`](Self::folder()) in cupertino_icons 1.0.0+.
    ///
    /// See also:
    ///
    ///  * [`folder`](Self::folder()), which is the equivalent of this icon for iOS versions later than or equal to iOS 7.
    pub fn folder_open() -> IconData {
        cupertino(0xf38a)
    }

    /// Cupertino icon for a trash bin for removing items. This icon is not filled in.
    /// This is the same icon as [`trash`](Self::trash()) and [`delete_simple`](Self::delete_simple()) in cupertino_icons 1.0.0+.
    ///
    /// See also:
    ///
    ///  * [`delete_solid`](Self::delete_solid()), which is similar, but filled in.
    pub fn delete() -> IconData {
        cupertino(0xf4c4)
    }

    /// Cupertino icon for a trash bin for removing items. This icon is filled in.
    /// This is the same icon as [`trash_fill`](Self::trash_fill()) in cupertino_icons 1.0.0+.
    ///
    /// See also:
    ///
    ///  * [`delete`](Self::delete()), which is similar, but not filled in.
    pub fn delete_solid() -> IconData {
        cupertino(0xf4c5)
    }

    /// Cupertino icon for a trash bin with minimal detail for removing items.
    /// This is the same icon as [`trash`](Self::trash()) and [`delete`](Self::delete()) in cupertino_icons 1.0.0+.
    ///
    /// See also:
    ///
    ///  * [`delete`](Self::delete()), which is the iOS 7 equivalent of this icon with richer detail.
    pub fn delete_simple() -> IconData {
        cupertino(0xf37f)
    }

    /// Cupertino icon for a simple pen.
    ///
    /// See also:
    ///
    ///  * [`pencil`](Self::pencil()), which is similar, but has less detail and looks like a pencil.
    pub fn pen() -> IconData {
        cupertino(0xf2bf)
    }

    /// Cupertino icon for a simple pencil.
    ///
    /// See also:
    ///
    ///  * [`pen`](Self::pen()), which is similar, but has more detail and looks like a pen.
    pub fn pencil() -> IconData {
        cupertino(0xf37e)
    }

    /// Cupertino icon for a box for writing and a pen on top (that indicates the writing). This icon is not filled in.
    /// This is the same icon as [`square_pencil`](Self::square_pencil()) in cupertino_icons 1.0.0+.
    ///
    /// See also:
    ///
    ///  * [`create_solid`](Self::create_solid()), which is similar, but filled in.
    ///  * [`pencil`](Self::pencil()), which is just a pencil.
    ///  * [`pen`](Self::pen()), which is just a pen.
    pub fn create() -> IconData {
        cupertino(0xf417)
    }

    /// Cupertino icon for a box for writing and a pen on top (that indicates the writing). This icon is filled in.
    /// This is the same icon as [`square_pencil_fill`](Self::square_pencil_fill()) in cupertino_icons 1.0.0+.
    ///
    /// See also:
    ///
    ///  * [`create`](Self::create()), which is similar, but not filled in.
    ///  * [`pencil`](Self::pencil()), which is just a pencil.
    ///  * [`pen`](Self::pen()), which is just a pen.
    pub fn create_solid() -> IconData {
        cupertino(0xf417)
    }

    /// Cupertino icon for an arrow on a circular path with its end pointing at its start.
    /// This is the same icon as [`arrow_clockwise`](Self::arrow_clockwise()), [`refresh_thin`](Self::refresh_thin()) and [`refresh_thick`](Self::refresh_thick()) in cupertino_icons 1.0.0+.
    ///
    /// See also:
    ///
    ///  * [`refresh_circled`](Self::refresh_circled()), which is this icon put in a circle.
    ///  * [`refresh_thin`](Self::refresh_thin()), which is an arrow of the same concept, but thinner and with a smaller gap in between its end and start.
    ///  * [`refresh_thick`](Self::refresh_thick()), which is similar, but rotated 45 degrees clockwise and thicker.
    ///  * [`refresh_bold`](Self::refresh_bold()), which is similar, but rotated 90 degrees clockwise and much thicker.
    pub fn refresh() -> IconData {
        cupertino(0xf49a)
    }

    /// Cupertino icon for an arrow on a circular path with its end pointing at its start surrounded by a circle. This is icon is not filled in.
    /// This is the same icon as [`arrow_clockwise_circle`](Self::arrow_clockwise_circle()) in cupertino_icons 1.0.0+.
    ///
    /// See also:
    ///
    ///  * [`refresh_circled_solid`](Self::refresh_circled_solid()), which is similar, but filled in.
    ///  * [`refresh`](Self::refresh()), which is the arrow of this icon without a circle.
    pub fn refresh_circled() -> IconData {
        cupertino(0xf49b)
    }

    /// Cupertino icon for an arrow on a circular path with its end pointing at its start surrounded by a circle. This is icon is filled in.
    /// This is the same icon as [`arrow_clockwise_circle_fill`](Self::arrow_clockwise_circle_fill()) in cupertino_icons 1.0.0+.
    ///
    /// See also:
    ///
    ///  * [`refresh_circled`](Self::refresh_circled()), which is similar, but not filled in.
    ///  * [`refresh`](Self::refresh()), which is the arrow of this icon filled in without a circle.
    pub fn refresh_circled_solid() -> IconData {
        cupertino(0xf49c)
    }

    /// Cupertino icon for an arrow on a circular path with its end pointing at its start.
    /// This is the same icon as [`arrow_clockwise`](Self::arrow_clockwise()), [`refresh`](Self::refresh()) and [`refresh_thick`](Self::refresh_thick()) in cupertino_icons 1.0.0+.
    ///
    /// See also:
    ///
    ///  * [`refresh`](Self::refresh()), which is an arrow of the same concept, but thicker and with a larger gap in between its end and start.
    pub fn refresh_thin() -> IconData {
        cupertino(0xf49d)
    }

    /// Cupertino icon for an arrow on a circular path with its end pointing at its start.
    /// This is the same icon as [`arrow_clockwise`](Self::arrow_clockwise()), [`refresh_thin`](Self::refresh_thin()) and [`refresh`](Self::refresh()) in cupertino_icons 1.0.0+.
    ///
    /// See also:
    ///
    ///  * [`refresh`](Self::refresh()), which is similar, but rotated 45 degrees anti-clockwise and thinner.
    ///  * [`refresh_bold`](Self::refresh_bold()), which is similar, but rotated 45 degrees clockwise and thicker.
    pub fn refresh_thick() -> IconData {
        cupertino(0xf3a8)
    }

    /// Cupertino icon for an arrow on a circular path with its end pointing at its start.
    /// This is the same icon as [`arrow_counterclockwise`](Self::arrow_counterclockwise()) in cupertino_icons 1.0.0+.
    ///
    /// See also:
    ///
    ///  * [`refresh_thick`](Self::refresh_thick()), which is similar, but rotated 45 degrees anti-clockwise and thinner.
    ///  * [`refresh`](Self::refresh()), which is similar, but rotated 90 degrees anti-clockwise and much thinner.
    pub fn refresh_bold() -> IconData {
        cupertino(0xf21c)
    }

    /// Cupertino icon for a cross of two diagonal lines from edge to edge crossing in an angle of 90 degrees, which is used for dismissal.
    /// This is the same icon as [`xmark`](Self::xmark()) and [`clear`](Self::clear()) in cupertino_icons 1.0.0+.
    ///
    /// See also:
    ///
    ///  * [`clear_circled`](Self::clear_circled()), which uses this cross as a blank space in a filled out circled.
    ///  * [`clear`](Self::clear()), which uses a thinner cross and is the iOS 7 equivalent of this icon.
    pub fn clear_thick() -> IconData {
        cupertino(0xf2d7)
    }

    /// Cupertino icon for a cross of two diagonal lines from edge to edge crossing in an angle of 90 degrees, which is used for dismissal, used as a blank space in a circle.
    /// This is the same icon as [`xmark_circle_fill`](Self::xmark_circle_fill()) and [`clear_circled_solid`](Self::clear_circled_solid()) in cupertino_icons 1.0.0+.
    ///
    /// See also:
    ///
    ///  * [`clear`](Self::clear()), which is equivalent to the cross of this icon without a circle.
    ///  * [`clear_circled_solid`](Self::clear_circled_solid()), which is similar, but uses a thinner cross.
    pub fn clear_thick_circled() -> IconData {
        cupertino(0xf36e)
    }

    /// Cupertino icon for a cross of two diagonal lines from edge to edge crossing in an angle of 90 degrees, which is used for dismissal.
    /// This is the same icon as [`xmark`](Self::xmark()) and [`clear_thick`](Self::clear_thick()) in cupertino_icons 1.0.0+.
    ///
    ///
    /// See also:
    ///
    ///  * [`clear_circled`](Self::clear_circled()), which consists of this cross and a circle surrounding it.
    ///  * [`clear`](Self::clear()), which uses a thicker cross and is the pre-iOS 7 equivalent of this icon.
    pub fn clear() -> IconData {
        cupertino(0xf404)
    }

    /// Cupertino icon for a cross of two diagonal lines from edge to edge crossing in an angle of 90 degrees, which is used for dismissal, surrounded by circle. This icon is not filled in.
    /// This is the same icon as [`xmark_circle`](Self::xmark_circle()) in cupertino_icons 1.0.0+.
    ///
    /// See also:
    ///
    ///  * [`clear_circled_solid`](Self::clear_circled_solid()), which is similar, but filled in.
    ///  * [`clear`](Self::clear()), which is the standalone cross of this icon.
    pub fn clear_circled() -> IconData {
        cupertino(0xf405)
    }

    /// Cupertino icon for a cross of two diagonal lines from edge to edge crossing in an angle of 90 degrees, which is used for dismissal, used as a blank space in a circle. This icon is filled in.
    /// This is the same icon as [`xmark_circle_fill`](Self::xmark_circle_fill()) and [`clear_thick_circled`](Self::clear_thick_circled()) in cupertino_icons 1.0.0+.
    ///
    /// See also:
    ///
    ///  * [`clear_circled`](Self::clear_circled()), which is similar, but not filled in.
    pub fn clear_circled_solid() -> IconData {
        cupertino(0xf406)
    }

    /// Cupertino icon for an two straight lines, one horizontal and one vertical, meeting in the middle, which is the equivalent of a plus sign.
    /// This is the same icon as [`plus`](Self::plus()) in cupertino_icons 1.0.0+.
    ///
    /// See also:
    ///
    ///  * [`plus_circled`](Self::plus_circled()), which is the pre-iOS 7 version of this icon with a thicker cross.
    ///  * [`add_circled`](Self::add_circled()), which consists of the plus and a circle around it.
    pub fn add() -> IconData {
        cupertino(0xf489)
    }

    /// Cupertino icon for an two straight lines, one horizontal and one vertical, meeting in the middle, which is the equivalent of a plus sign, surrounded by a circle. This icon is not filled in.
    /// This is the same icon as [`plus_circle`](Self::plus_circle()) in cupertino_icons 1.0.0+.
    ///
    /// See also:
    ///
    ///  * [`plus_circled`](Self::plus_circled()), which is the pre-iOS 7 version of this icon with a thicker cross and a filled in circle.
    ///  * [`add_circled_solid`](Self::add_circled_solid()), which is similar, but filled in.
    pub fn add_circled() -> IconData {
        cupertino(0xf48a)
    }

    /// Cupertino icon for an two straight lines, one horizontal and one vertical, meeting in the middle, which is the equivalent of a plus sign, surrounded by a circle. This icon is not filled in.
    /// This is the same icon as [`plus_circle_fill`](Self::plus_circle_fill()) in cupertino_icons 1.0.0+.
    ///
    /// See also:
    ///
    ///  * [`plus_circled`](Self::plus_circled()), which is the pre-iOS 7 version of this icon with a thicker cross.
    ///  * [`add_circled`](Self::add_circled()), which is similar, but not filled in.
    pub fn add_circled_solid() -> IconData {
        cupertino(0xf48b)
    }

    /// Cupertino icon for a gear with eight cogs. This icon is not filled in.
    /// This is the same icon as [`gear_alt`](Self::gear_alt()) and [`gear_big`](Self::gear_big()) in cupertino_icons 1.0.0+.
    ///
    /// See also:
    ///
    ///  * [`gear_solid`](Self::gear_solid()), which is similar, but filled in.
    ///  * [`gear_big`](Self::gear_big()), which is the pre-iOS 7 version of this icon and appears bigger because of fewer and bigger cogs.
    ///  * [`settings`](Self::settings()), which is another cogwheel with a different design.
    pub fn gear() -> IconData {
        cupertino(0xf43c)
    }

    /// Cupertino icon for a gear with eight cogs. This icon is filled in.
    /// This is the same icon as [`gear_alt_fill`](Self::gear_alt_fill()) in cupertino_icons 1.0.0+.
    ///
    /// See also:
    ///
    ///  * [`gear`](Self::gear()), which is similar, but not filled in.
    ///  * [`settings_solid`](Self::settings_solid()), which is another cogwheel with a different design.
    pub fn gear_solid() -> IconData {
        cupertino(0xf43d)
    }

    /// Cupertino icon for a gear with six cogs.
    /// This is the same icon as [`gear_alt`](Self::gear_alt()) and [`gear`](Self::gear()) in cupertino_icons 1.0.0+.
    ///
    /// See also:
    ///
    ///  * [`gear`](Self::gear()), which is the iOS 7 version of this icon and appears smaller because of more and larger cogs.
    ///  * [`settings_solid`](Self::settings_solid()), which is another cogwheel with a different design.
    pub fn gear_big() -> IconData {
        cupertino(0xf2f7)
    }

    /// Cupertino icon for a cogwheel with many cogs and decoration in the middle. This icon is not filled in.
    ///
    /// See also:
    ///
    ///  * [`settings_solid`](Self::settings_solid()), which is similar, but filled in.
    ///  * [`gear`](Self::gear()), which is another cogwheel with a different design.
    pub fn settings() -> IconData {
        cupertino(0xf411)
    }

    /// Cupertino icon for a cogwheel with many cogs and decoration in the middle. This icon is filled in.
    ///
    /// See also:
    ///
    ///  * [`settings`](Self::settings()), which is similar, but not filled in.
    ///  * [`gear_solid`](Self::gear_solid()), which is another cogwheel with a different design.
    pub fn settings_solid() -> IconData {
        cupertino(0xf412)
    }

    /// Cupertino icon for a symbol representing a solid single musical note.
    ///
    /// See also:
    ///
    ///  * [`double_music_note`](Self::double_music_note()), which is similar, but with 2 connected notes.
    pub fn music_note() -> IconData {
        cupertino(0xf46b)
    }

    /// Cupertino icon for a symbol representing 2 connected musical notes.
    /// This is the same icon as [`music_note_2`](Self::music_note_2()) in cupertino_icons 1.0.0+.
    ///
    /// See also:
    ///
    ///  * [`music_note`](Self::music_note()), which is similar, but with a single note.
    pub fn double_music_note() -> IconData {
        cupertino(0xf46c)
    }

    /// Cupertino icon for a triangle facing to the right. This icon is not filled in.
    /// This is the same icon as [`play`](Self::play()) in cupertino_icons 1.0.0+.
    ///
    /// See also:
    ///
    ///  * [`play_arrow_solid`](Self::play_arrow_solid()), which is similar, but filled in.
    pub fn play_arrow() -> IconData {
        cupertino(0xf487)
    }

    /// Cupertino icon for a triangle facing to the right. This icon is filled in.
    /// This is the same icon as [`play_fill`](Self::play_fill()) in cupertino_icons 1.0.0+.
    ///
    /// See also:
    ///
    ///  * [`play_arrow`](Self::play_arrow()), which is similar, but not filled in.
    pub fn play_arrow_solid() -> IconData {
        cupertino(0xf488)
    }

    /// Cupertino icon for an two vertical rectangles. This icon is not filled in.
    ///
    /// See also:
    ///
    ///  * [`pause_solid`](Self::pause_solid()), which is similar, but filled in.
    pub fn pause() -> IconData {
        cupertino(0xf477)
    }

    /// Cupertino icon for an two vertical rectangles. This icon is filled in.
    /// This is the same icon as [`pause_fill`](Self::pause_fill()) in cupertino_icons 1.0.0+.
    ///
    /// See also:
    ///
    ///  * [`pause`](Self::pause()), which is similar, but not filled in.
    pub fn pause_solid() -> IconData {
        cupertino(0xf478)
    }

    /// Cupertino icon for the infinity symbol.
    /// This is the same icon as [`infinite`](Self::infinite()) and [`loop_thick`](Self::loop_thick()) in cupertino_icons 1.0.0+.
    ///
    /// See also:
    ///
    ///  * [`loop_thick`](Self::loop_thick()), which is similar, but thicker.
    pub fn r#loop() -> IconData {
        cupertino(0xf449)
    }

    /// Cupertino icon for the infinity symbol.
    /// This is the same icon as [`infinite`](Self::infinite()) and [`loop`](Self::loop()) in cupertino_icons 1.0.0+.
    ///
    /// See also:
    ///
    ///  * [`loop`](Self::loop()), which is similar, but thinner.
    pub fn loop_thick() -> IconData {
        cupertino(0xf44a)
    }

    /// Cupertino icon for a speaker with a single small sound wave.
    /// This is the same icon as [`speaker_1_fill`](Self::speaker_1_fill()) in cupertino_icons 1.0.0+.
    ///
    /// See also:
    ///
    ///  * [`volume_mute`](Self::volume_mute()), which is similar, but has no sound waves.
    ///  * [`volume_off`](Self::volume_off()), which is similar, but with an additional larger sound wave and a diagonal line crossing the whole icon.
    ///  * [`volume_up`](Self::volume_up()), which has an additional larger sound wave next to the small one.
    pub fn volume_down() -> IconData {
        cupertino(0xf3b7)
    }

    /// Cupertino icon for a speaker symbol.
    /// This is the same icon as [`speaker_fill`](Self::speaker_fill()) in cupertino_icons 1.0.0+.
    ///
    /// See also:
    ///
    ///  * [`volume_down`](Self::volume_down()), which is similar, but adds a small sound wave.
    ///  * [`volume_off`](Self::volume_off()), which is similar, but adds a small and a large sound wave and a diagonal line crossing the whole icon.
    ///  * [`volume_up`](Self::volume_up()), which is similar, but has a small and a large sound wave.
    pub fn volume_mute() -> IconData {
        cupertino(0xf3b8)
    }

    /// Cupertino icon for a speaker with a small and a large sound wave and a diagonal line crossing the whole icon.
    /// This is the same icon as [`speaker_slash_fill`](Self::speaker_slash_fill()) in cupertino_icons 1.0.0+.
    ///
    /// See also:
    ///
    ///  * [`volume_down`](Self::volume_down()), which is similar, but not crossed out and only has the small wave.
    ///  * [`volume_mute`](Self::volume_mute()), which is similar, but not crossed out.
    ///  * [`volume_up`](Self::volume_up()), which is the version of this icon that is not crossed out.
    pub fn volume_off() -> IconData {
        cupertino(0xf3b9)
    }

    /// Cupertino icon for a speaker with a small and a large sound wave.
    /// This is the same icon as [`speaker_3_fill`](Self::speaker_3_fill()) in cupertino_icons 1.0.0+.
    ///
    /// See also:
    ///
    ///  * [`volume_down`](Self::volume_down()), which is similar, but only has the small sound wave.
    ///  * [`volume_mute`](Self::volume_mute()), which is similar, but has no sound waves.
    ///  * [`volume_off`](Self::volume_off()), which is the crossed out version of this icon.
    pub fn volume_up() -> IconData {
        cupertino(0xf3ba)
    }

    /// Cupertino icon for all four corners of a square facing inwards.
    /// This is the same icon as [`arrow_up_left_arrow_down_right`](Self::arrow_up_left_arrow_down_right()) in cupertino_icons 1.0.0+.
    ///
    /// See also:
    ///
    ///  * [`fullscreen_exit`](Self::fullscreen_exit()), which is similar, but has the corners facing outwards.
    pub fn fullscreen() -> IconData {
        cupertino(0xf386)
    }

    /// Cupertino icon for all four corners of a square facing outwards.
    /// This is the same icon as [`arrow_down_right_arrow_up_left`](Self::arrow_down_right_arrow_up_left()) in cupertino_icons 1.0.0+.
    ///
    /// See also:
    ///
    ///  * [`fullscreen`](Self::fullscreen()), which is similar, but has the corners facing inwards.
    pub fn fullscreen_exit() -> IconData {
        cupertino(0xf37d)
    }

    /// Cupertino icon for a filled in microphone with a diagonal line crossing it.
    /// This is the same icon as [`mic_slash`](Self::mic_slash()) in cupertino_icons 1.0.0+.
    ///
    /// See also:
    ///
    ///  * [`mic`](Self::mic()), which is similar, but not filled in and without a diagonal line.
    ///  * [`mic_solid`](Self::mic_solid()), which is similar, but without a diagonal line.
    pub fn mic_off() -> IconData {
        cupertino(0xf45f)
    }

    /// Cupertino icon for a microphone.
    ///
    /// See also:
    ///
    ///  * [`mic_solid`](Self::mic_solid()), which is similar, but filled in.
    ///  * [`mic_off`](Self::mic_off()), which is similar, but filled in and with a diagonal line crossing the icon.
    pub fn mic() -> IconData {
        cupertino(0xf460)
    }

    /// Cupertino icon for a filled in microphone.
    /// This is the same icon as [`mic_fill`](Self::mic_fill()) in cupertino_icons 1.0.0+.
    ///
    /// See also:
    ///
    ///  * [`mic`](Self::mic()), which is similar, but not filled in.
    ///  * [`mic_off`](Self::mic_off()), which is similar, but with a diagonal line crossing the icon.
    pub fn mic_solid() -> IconData {
        cupertino(0xf461)
    }

    /// Cupertino icon for a circle with a dotted clock face inside with hands showing 10:30.
    /// This is the same icon as [`time`](Self::time()) in cupertino_icons 1.0.0+.
    ///
    /// See also:
    ///
    ///  * [`clock_solid`](Self::clock_solid()), which is similar, but filled in.
    ///  * [`time`](Self::time()), which is similar, but without dots on the clock face.
    ///  * [`time_solid`](Self::time_solid()), which is similar, but filled in and without dots on the clock face.
    pub fn clock() -> IconData {
        cupertino(0xf4be)
    }

    /// Cupertino icon for a filled in circle with a dotted clock face inside with hands showing 10:30.
    /// This is the same icon as [`clock_fill`](Self::clock_fill()) and [`time_solid`](Self::time_solid()) in cupertino_icons 1.0.0+.
    ///
    /// See also:
    ///
    ///  * [`clock`](Self::clock()), which is similar, but not filled in.
    ///  * [`time`](Self::time()), which is similar, but not filled in and without dots on the clock face.
    ///  * [`time_solid`](Self::time_solid()), which is similar, but without dots on the clock face.
    pub fn clock_solid() -> IconData {
        cupertino(0xf4bf)
    }

    /// Cupertino icon for a circle with a 90 degree angle shape in the center, resembling a clock with hands showing 09:00.
    /// This is the same icon as [`clock`](Self::clock()) in cupertino_icons 1.0.0+.
    ///
    /// See also:
    ///
    ///  * [`time_solid`](Self::time_solid()), which is similar, but filled in.
    ///  * [`clock`](Self::clock()), which is similar, but with dots on the clock face.
    ///  * [`clock_solid`](Self::clock_solid()), which is similar, but filled in and with dots on the clock face.
    pub fn time() -> IconData {
        cupertino(0xf402)
    }

    /// Cupertino icon for a filled in circle with a 90 degree angle shape in the center, resembling a clock with hands showing 09:00.
    /// This is the same icon as [`clock_fill`](Self::clock_fill()) and [`clock_solid`](Self::clock_solid()) in cupertino_icons 1.0.0+.
    ///
    /// See also:
    ///
    ///  * [`time`](Self::time()), which is similar, but not filled in.
    ///  * [`clock`](Self::clock()), which is similar, but not filled in and with dots on the clock face.
    ///  * [`clock_solid`](Self::clock_solid()), which is similar, but with dots on the clock face.
    pub fn time_solid() -> IconData {
        cupertino(0xf403)
    }

    /// Cupertino icon for an unlocked padlock.
    /// This is the same icon as [`lock`](Self::lock()) in cupertino_icons 1.0.0+.
    ///
    /// See also:
    ///
    ///  * [`padlock_solid`](Self::padlock_solid()), which is similar, but filled in.
    pub fn padlock() -> IconData {
        cupertino(0xf4c8)
    }

    /// Cupertino icon for an unlocked padlock.
    /// This is the same icon as [`lock_fill`](Self::lock_fill()) in cupertino_icons 1.0.0+.
    ///
    /// See also:
    ///
    ///  * [`padlock`](Self::padlock()), which is similar, but not filled in.
    pub fn padlock_solid() -> IconData {
        cupertino(0xf4c9)
    }

    /// Cupertino icon for an open eye.
    ///
    /// See also:
    ///
    ///  * [`eye_solid`](Self::eye_solid()), which is similar, but filled in.
    pub fn eye() -> IconData {
        cupertino(0xf424)
    }

    /// Cupertino icon for an open eye.
    /// This is the same icon as [`eye_fill`](Self::eye_fill()) in cupertino_icons 1.0.0+.
    ///
    /// See also:
    ///
    ///  * [`eye`](Self::eye()), which is similar, but not filled in.
    pub fn eye_solid() -> IconData {
        cupertino(0xf425)
    }

    /// Cupertino icon for a single person. This icon is not filled in.
    ///
    /// See also:
    ///
    ///  * [`person_solid`](Self::person_solid()), which is similar, but filled in.
    ///  * [`person_add`](Self::person_add()), which has an additional plus sign next to the person.
    ///  * [`group`](Self::group()), which consists of three people.
    pub fn person() -> IconData {
        cupertino(0xf47d)
    }

    /// Cupertino icon for a single person. This icon is filled in.
    /// This is the same icon as [`person_fill`](Self::person_fill()) in cupertino_icons 1.0.0+.
    ///
    /// See also:
    ///
    ///  * [`person`](Self::person()), which is similar, but not filled in.
    ///  * [`person_add_solid`](Self::person_add_solid()), which has an additional plus sign next to the person.
    ///  * [`group_solid`](Self::group_solid()), which consists of three people.
    pub fn person_solid() -> IconData {
        cupertino(0xf47e)
    }

    /// Cupertino icon for a single person with a plus sign next to it. This icon is not filled in.
    /// This is the same icon as [`person_badge_plus`](Self::person_badge_plus()) in cupertino_icons 1.0.0+.x
    ///
    /// See also:
    ///
    ///  * [`person_add_solid`](Self::person_add_solid()), which is similar, but filled in.
    ///  * [`person`](Self::person()), which is just the person.
    ///  * [`group`](Self::group()), which consists of three people.
    pub fn person_add() -> IconData {
        cupertino(0xf47f)
    }

    /// Cupertino icon for a single person with a plus sign next to it. This icon is filled in.
    /// This is the same icon as [`person_badge_plus_fill`](Self::person_badge_plus_fill()) in cupertino_icons 1.0.0+.
    ///
    /// See also:
    ///
    ///  * [`person_add`](Self::person_add()), which is similar, but not filled in.
    ///  * [`person_solid`](Self::person_solid()), which is just the person.
    ///  * [`group_solid`](Self::group_solid()), which consists of three people.
    pub fn person_add_solid() -> IconData {
        cupertino(0xf480)
    }

    /// Cupertino icon for a group of three people. This icon is not filled in.
    /// This is the same icon as [`person_3`](Self::person_3()) in cupertino_icons 1.0.0+.
    ///
    /// See also:
    ///
    ///  * [`group_solid`](Self::group_solid()), which is similar, but filled in.
    ///  * [`person`](Self::person()), which is just a single person.
    pub fn group() -> IconData {
        cupertino(0xf47b)
    }

    /// Cupertino icon for a group of three people. This icon is filled in.
    /// This is the same icon as [`person_3_fill`](Self::person_3_fill()) in cupertino_icons 1.0.0+.
    ///
    /// See also:
    ///
    ///  * [`group`](Self::group()), which is similar, but not filled in.
    ///  * [`person_solid`](Self::person_solid()), which is just a single person.
    pub fn group_solid() -> IconData {
        cupertino(0xf47c)
    }

    /// Cupertino icon for the outline of a closed mail envelope.
    /// This is the same icon as [`envelope`](Self::envelope()) in cupertino_icons 1.0.0+.
    pub fn mail() -> IconData {
        cupertino(0xf422)
    }

    /// Cupertino icon for a closed mail envelope. This icon is filled in.
    /// This is the same icon as [`envelope_fill`](Self::envelope_fill()) in cupertino_icons 1.0.0+.
    pub fn mail_solid() -> IconData {
        cupertino(0xf423)
    }

    /// Cupertino icon for a location pin.
    pub fn location() -> IconData {
        cupertino(0xf6ee)
    }

    /// Cupertino icon for a location pin. This icon is filled in.
    /// This is the same icon as [`placemark_fill`](Self::placemark_fill()) in cupertino_icons 1.0.0+.
    pub fn location_solid() -> IconData {
        cupertino(0xf456)
    }

    /// Cupertino icon for the outline of a sticker tag.
    /// This is the same icon as [`tags`](Self::tags()) in cupertino_icons 1.0.0+.
    ///
    /// See also:
    ///
    ///  * [`tags`](Self::tags()), similar but with 2 overlapping tags.
    pub fn tag() -> IconData {
        cupertino(0xf48c)
    }

    /// Cupertino icon for a sticker tag. This icon is filled in.
    /// This is the same icon as [`tag_fill`](Self::tag_fill()) and [`tags_solid`](Self::tags_solid()) in cupertino_icons 1.0.0+.
    ///
    /// See also:
    ///
    ///  * [`tags_solid`](Self::tags_solid()), similar but with 2 overlapping tags.
    pub fn tag_solid() -> IconData {
        cupertino(0xf48d)
    }

    /// Cupertino icon for outlines of 2 overlapping sticker tags.
    /// This is the same icon as [`tag`](Self::tag()) in cupertino_icons 1.0.0+.
    ///
    /// See also:
    ///
    ///  * [`tag`](Self::tag()), similar but with only one tag.
    pub fn tags() -> IconData {
        cupertino(0xf48e)
    }

    /// Cupertino icon for 2 overlapping sticker tags. This icon is filled in.
    /// This is the same icon as [`tag_fill`](Self::tag_fill()) and [`tag_solid`](Self::tag_solid()) in cupertino_icons 1.0.0+.
    ///
    /// See also:
    ///
    ///  * [`tag_solid`](Self::tag_solid()), similar but with only one tag.
    pub fn tags_solid() -> IconData {
        cupertino(0xf48f)
    }

    /// Cupertino icon for a filled in bus.
    /// This icon is available in cupertino_icons 1.0.0+ for backward
    /// compatibility but not part of Apple icons' aesthetics.
    pub fn bus() -> IconData {
        cupertino(0xf36d)
    }

    /// Cupertino icon for a filled in car.
    /// This is the same icon as [`car_fill`](Self::car_fill()) in cupertino_icons 1.0.0+.
    ///
    /// See also:
    ///
    ///  * [`car_detailed`](Self::car_detailed()), similar, but a more detailed and realistic representation.
    pub fn car() -> IconData {
        cupertino(0xf36f)
    }

    /// Cupertino icon for a filled in detailed, realistic car.
    ///
    /// See also:
    ///
    ///  * [`car`](Self::car()), similar, but a more simple representation.
    ///    This icon is available in cupertino_icons 1.0.0+ for backward
    ///    compatibility but not part of Apple icons' aesthetics.
    pub fn car_detailed() -> IconData {
        cupertino(0xf2c1)
    }

    /// Cupertino icon for a filled in train with a window divided in half and two headlights.
    /// This icon is available in cupertino_icons 1.0.0+ for backward
    /// compatibility but not part of Apple icons' aesthetics.
    ///
    /// See also:
    ///
    ///  * [`train_style_two`](Self::train_style_two()), similar, but with a full, undivided window and a single, centered headlight.
    pub fn train_style_one() -> IconData {
        cupertino(0xf3af)
    }

    /// Cupertino icon for a filled in train with a window and a single, centered headlight.
    /// This icon is available in cupertino_icons 1.0.0+ for backward
    /// compatibility but not part of Apple icons' aesthetics.
    ///
    /// See also:
    ///
    ///  * [`train_style_one`](Self::train_style_one()), similar, but with a with a window divided in half and two headlights.
    pub fn train_style_two() -> IconData {
        cupertino(0xf3b4)
    }

    /// Cupertino icon for an outlined paw.
    ///
    /// See also:
    ///
    ///  * [`paw_solid`](Self::paw_solid()), similar, but filled in.
    pub fn paw() -> IconData {
        cupertino(0xf479)
    }

    /// Cupertino icon for a filled in paw.
    /// This is the same icon as [`paw`](Self::paw()) in cupertino_icons 1.0.0+.
    ///
    /// See also:
    ///
    ///  * [`paw`](Self::paw()), similar, but not filled in.
    pub fn paw_solid() -> IconData {
        cupertino(0xf47a)
    }

    /// Cupertino icon for an outlined game controller.
    /// This is the same icon as [`gamecontroller`](Self::gamecontroller()) in cupertino_icons 1.0.0+.
    ///
    /// See also:
    ///
    ///  * [`game_controller_solid`](Self::game_controller_solid()), similar, but filled in.
    pub fn game_controller() -> IconData {
        cupertino(0xf43a)
    }

    /// Cupertino icon for a filled in game controller.
    /// This is the same icon as [`gamecontroller_fill`](Self::gamecontroller_fill()) in cupertino_icons 1.0.0+.
    ///
    /// See also:
    ///
    ///  * [`game_controller`](Self::game_controller()), similar, but not filled in.
    pub fn game_controller_solid() -> IconData {
        cupertino(0xf43b)
    }

    /// Cupertino icon for an outlined lab flask.
    /// This icon is available in cupertino_icons 1.0.0+ for backward
    /// compatibility but not part of Apple icons' aesthetics.
    ///
    /// See also:
    ///
    ///  * [`lab_flask_solid`](Self::lab_flask_solid()), similar, but filled in.
    pub fn lab_flask() -> IconData {
        cupertino(0xf430)
    }

    /// Cupertino icon for a filled in lab flask.
    /// This icon is available in cupertino_icons 1.0.0+ for backward
    /// compatibility but not part of Apple icons' aesthetics.
    ///
    /// See also:
    ///
    ///  * [`lab_flask`](Self::lab_flask()), similar, but not filled in.
    pub fn lab_flask_solid() -> IconData {
        cupertino(0xf431)
    }

    /// Cupertino icon for an outlined heart shape. Can be used to indicate like or favorite states.
    ///
    /// See also:
    ///
    ///  * [`heart_solid`](Self::heart_solid()), same shape, but filled in.
    pub fn heart() -> IconData {
        cupertino(0xf442)
    }

    /// Cupertino icon for a filled heart shape. Can be used to indicate like or favorite states.
    /// This is the same icon as [`heart_fill`](Self::heart_fill()) in cupertino_icons 1.0.0+.
    ///
    /// See also:
    ///
    ///  * [`heart`](Self::heart()), same shape, but not filled in.
    pub fn heart_solid() -> IconData {
        cupertino(0xf443)
    }

    /// Cupertino icon for an outlined bell. Can be used to represent notifications.
    ///
    /// See also:
    ///
    ///  * [`bell_solid`](Self::bell_solid()), same shape, but filled in.
    pub fn bell() -> IconData {
        cupertino(0xf3e1)
    }

    /// Cupertino icon for a filled bell. Can be used represent notifications.
    /// This is the same icon as [`bell_fill`](Self::bell_fill()) in cupertino_icons 1.0.0+.
    ///
    /// See also:
    ///
    ///  * [`bell`](Self::bell()), same shape, but not filled in.
    pub fn bell_solid() -> IconData {
        cupertino(0xf3e2)
    }

    /// Cupertino icon for an outlined folded newspaper icon.
    /// This icon is available in cupertino_icons 1.0.0+ for backward
    /// compatibility but not part of Apple icons' aesthetics.
    ///
    /// See also:
    ///
    ///  * [`news_solid`](Self::news_solid()), same shape, but filled in.
    pub fn news() -> IconData {
        cupertino(0xf471)
    }

    /// Cupertino icon for a filled folded newspaper icon.
    /// This icon is available in cupertino_icons 1.0.0+ for backward
    /// compatibility but not part of Apple icons' aesthetics.
    ///
    /// See also:
    ///
    ///  * [`news`](Self::news()), same shape, but not filled in.
    pub fn news_solid() -> IconData {
        cupertino(0xf472)
    }

    /// Cupertino icon for an outlined brightness icon.
    /// This is the same icon as [`sun_max`](Self::sun_max()) in cupertino_icons 1.0.0+.
    ///
    /// See also:
    ///
    ///  * [`brightness_solid`](Self::brightness_solid()), same shape, but filled in.
    pub fn brightness() -> IconData {
        cupertino(0xf4b6)
    }

    /// Cupertino icon for a filled in brightness icon.
    /// This is the same icon as [`sun_max_fill`](Self::sun_max_fill()) in cupertino_icons 1.0.0+.
    ///
    /// See also:
    ///
    ///  * [`brightness`](Self::brightness()), same shape, but not filled in.
    pub fn brightness_solid() -> IconData {
        cupertino(0xf4b7)
    }

    // END LEGACY PRE SF SYMBOLS NAMES
    // ===========================================================================
    // ===========================================================================
    // BEGIN GENERATED SF SYMBOLS NAMES

    /// Cupertino icon named "airplane". Available on cupertino_icons package 1.0.0+ only.
    pub fn airplane() -> IconData {
        cupertino(0xf4d4)
    }

    /// Cupertino icon named "alarm". Available on cupertino_icons package 1.0.0+ only.
    pub fn alarm() -> IconData {
        cupertino(0xf4d5)
    }

    /// Cupertino icon named "alarm_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn alarm_fill() -> IconData {
        cupertino(0xf4d6)
    }

    /// Cupertino icon named "alt". Available on cupertino_icons package 1.0.0+ only.
    pub fn alt() -> IconData {
        cupertino(0xf4d7)
    }

    /// Cupertino icon named "ant". Available on cupertino_icons package 1.0.0+ only.
    pub fn ant() -> IconData {
        cupertino(0xf4d8)
    }

    /// Cupertino icon named "ant_circle". Available on cupertino_icons package 1.0.0+ only.
    pub fn ant_circle() -> IconData {
        cupertino(0xf4d9)
    }

    /// Cupertino icon named "ant_circle_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn ant_circle_fill() -> IconData {
        cupertino(0xf4da)
    }

    /// Cupertino icon named "ant_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn ant_fill() -> IconData {
        cupertino(0xf4db)
    }

    /// Cupertino icon named "antenna_radiowaves_left_right". Available on cupertino_icons package 1.0.0+ only.
    pub fn antenna_radiowaves_left_right() -> IconData {
        cupertino(0xf4dc)
    }

    /// Cupertino icon named "app". Available on cupertino_icons package 1.0.0+ only.
    pub fn app() -> IconData {
        cupertino(0xf4dd)
    }

    /// Cupertino icon named "app_badge". Available on cupertino_icons package 1.0.0+ only.
    pub fn app_badge() -> IconData {
        cupertino(0xf4de)
    }

    /// Cupertino icon named "app_badge_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn app_badge_fill() -> IconData {
        cupertino(0xf4df)
    }

    /// Cupertino icon named "app_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn app_fill() -> IconData {
        cupertino(0xf4e0)
    }

    /// Cupertino icon named "archivebox". Available on cupertino_icons package 1.0.0+ only.
    pub fn archivebox() -> IconData {
        cupertino(0xf4e1)
    }

    /// Cupertino icon named "archivebox_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn archivebox_fill() -> IconData {
        cupertino(0xf4e2)
    }

    /// Cupertino icon named "arrow_2_circlepath". Available on cupertino_icons package 1.0.0+ only.
    pub fn arrow_2_circlepath() -> IconData {
        cupertino(0xf4e3)
    }

    /// Cupertino icon named "arrow_2_circlepath_circle". Available on cupertino_icons package 1.0.0+ only.
    pub fn arrow_2_circlepath_circle() -> IconData {
        cupertino(0xf4e4)
    }

    /// Cupertino icon named "arrow_2_circlepath_circle_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn arrow_2_circlepath_circle_fill() -> IconData {
        cupertino(0xf4e5)
    }

    /// Cupertino icon named "arrow_2_squarepath". Available on cupertino_icons package 1.0.0+ only.
    pub fn arrow_2_squarepath() -> IconData {
        cupertino(0xf4e6)
    }

    /// Cupertino icon named "arrow_3_trianglepath". Available on cupertino_icons package 1.0.0+ only.
    pub fn arrow_3_trianglepath() -> IconData {
        cupertino(0xf4e7)
    }

    /// Cupertino icon named "arrow_branch". Available on cupertino_icons package 1.0.0+ only.
    pub fn arrow_branch() -> IconData {
        cupertino(0xf4e8)
    }

    /// Cupertino icon named "arrow_clockwise". Available on cupertino_icons package 1.0.0+ only.
    /// This is the same icon as [`refresh`](Self::refresh()) which is available in cupertino_icons 0.1.3.
    /// This is the same icon as [`refresh_thin`](Self::refresh_thin()) which is available in cupertino_icons 0.1.3.
    /// This is the same icon as [`refresh_thick`](Self::refresh_thick()) which is available in cupertino_icons 0.1.3.
    pub fn arrow_clockwise() -> IconData {
        cupertino(0xf49a)
    }

    /// Cupertino icon named "arrow_clockwise_circle". Available on cupertino_icons package 1.0.0+ only.
    /// This is the same icon as [`refresh_circled`](Self::refresh_circled()) which is available in cupertino_icons 0.1.3.
    pub fn arrow_clockwise_circle() -> IconData {
        cupertino(0xf49b)
    }

    /// Cupertino icon named "arrow_clockwise_circle_fill". Available on cupertino_icons package 1.0.0+ only.
    /// This is the same icon as [`refresh_circled_solid`](Self::refresh_circled_solid()) which is available in cupertino_icons 0.1.3.
    pub fn arrow_clockwise_circle_fill() -> IconData {
        cupertino(0xf49c)
    }

    /// Cupertino icon named "arrow_counterclockwise". Available on cupertino_icons package 1.0.0+ only.
    /// This is the same icon as [`restart`](Self::restart()) which is available in cupertino_icons 0.1.3.
    /// This is the same icon as [`refresh_bold`](Self::refresh_bold()) which is available in cupertino_icons 0.1.3.
    pub fn arrow_counterclockwise() -> IconData {
        cupertino(0xf21c)
    }

    /// Cupertino icon named "arrow_counterclockwise_circle". Available on cupertino_icons package 1.0.0+ only.
    pub fn arrow_counterclockwise_circle() -> IconData {
        cupertino(0xf4e9)
    }

    /// Cupertino icon named "arrow_counterclockwise_circle_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn arrow_counterclockwise_circle_fill() -> IconData {
        cupertino(0xf4ea)
    }

    /// Cupertino icon named "arrow_down". Available on cupertino_icons package 1.0.0+ only.
    /// This is the same icon as [`down_arrow`](Self::down_arrow()) which is available in cupertino_icons 0.1.3.
    pub fn arrow_down() -> IconData {
        cupertino(0xf35d)
    }

    /// Cupertino icon named "arrow_down_circle". Available on cupertino_icons package 1.0.0+ only.
    pub fn arrow_down_circle() -> IconData {
        cupertino(0xf4eb)
    }

    /// Cupertino icon named "arrow_down_circle_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn arrow_down_circle_fill() -> IconData {
        cupertino(0xf4ec)
    }

    /// Cupertino icon named "arrow_down_doc". Available on cupertino_icons package 1.0.0+ only.
    pub fn arrow_down_doc() -> IconData {
        cupertino(0xf4ed)
    }

    /// Cupertino icon named "arrow_down_doc_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn arrow_down_doc_fill() -> IconData {
        cupertino(0xf4ee)
    }

    /// Cupertino icon named "arrow_down_left". Available on cupertino_icons package 1.0.0+ only.
    pub fn arrow_down_left() -> IconData {
        cupertino(0xf4ef)
    }

    /// Cupertino icon named "arrow_down_left_circle". Available on cupertino_icons package 1.0.0+ only.
    pub fn arrow_down_left_circle() -> IconData {
        cupertino(0xf4f0)
    }

    /// Cupertino icon named "arrow_down_left_circle_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn arrow_down_left_circle_fill() -> IconData {
        cupertino(0xf4f1)
    }

    /// Cupertino icon named "arrow_down_left_square". Available on cupertino_icons package 1.0.0+ only.
    pub fn arrow_down_left_square() -> IconData {
        cupertino(0xf4f2)
    }

    /// Cupertino icon named "arrow_down_left_square_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn arrow_down_left_square_fill() -> IconData {
        cupertino(0xf4f3)
    }

    /// Cupertino icon named "arrow_down_right". Available on cupertino_icons package 1.0.0+ only.
    pub fn arrow_down_right() -> IconData {
        cupertino(0xf4f4)
    }

    /// Cupertino icon named "arrow_down_right_arrow_up_left". Available on cupertino_icons package 1.0.0+ only.
    /// This is the same icon as [`fullscreen_exit`](Self::fullscreen_exit()) which is available in cupertino_icons 0.1.3.
    pub fn arrow_down_right_arrow_up_left() -> IconData {
        cupertino(0xf37d)
    }

    /// Cupertino icon named "arrow_down_right_circle". Available on cupertino_icons package 1.0.0+ only.
    pub fn arrow_down_right_circle() -> IconData {
        cupertino(0xf4f5)
    }

    /// Cupertino icon named "arrow_down_right_circle_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn arrow_down_right_circle_fill() -> IconData {
        cupertino(0xf4f6)
    }

    /// Cupertino icon named "arrow_down_right_square". Available on cupertino_icons package 1.0.0+ only.
    pub fn arrow_down_right_square() -> IconData {
        cupertino(0xf4f7)
    }

    /// Cupertino icon named "arrow_down_right_square_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn arrow_down_right_square_fill() -> IconData {
        cupertino(0xf4f8)
    }

    /// Cupertino icon named "arrow_down_square". Available on cupertino_icons package 1.0.0+ only.
    pub fn arrow_down_square() -> IconData {
        cupertino(0xf4f9)
    }

    /// Cupertino icon named "arrow_down_square_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn arrow_down_square_fill() -> IconData {
        cupertino(0xf4fa)
    }

    /// Cupertino icon named "arrow_down_to_line". Available on cupertino_icons package 1.0.0+ only.
    pub fn arrow_down_to_line() -> IconData {
        cupertino(0xf4fb)
    }

    /// Cupertino icon named "arrow_down_to_line_alt". Available on cupertino_icons package 1.0.0+ only.
    pub fn arrow_down_to_line_alt() -> IconData {
        cupertino(0xf4fc)
    }

    /// Cupertino icon named "arrow_left". Available on cupertino_icons package 1.0.0+ only.
    pub fn arrow_left() -> IconData {
        cupertino(0xf4fd)
    }

    /// Cupertino icon named "arrow_left_circle". Available on cupertino_icons package 1.0.0+ only.
    pub fn arrow_left_circle() -> IconData {
        cupertino(0xf4fe)
    }

    /// Cupertino icon named "arrow_left_circle_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn arrow_left_circle_fill() -> IconData {
        cupertino(0xf4ff)
    }

    /// Cupertino icon named "arrow_left_right". Available on cupertino_icons package 1.0.0+ only.
    pub fn arrow_left_right() -> IconData {
        cupertino(0xf500)
    }

    /// Cupertino icon named "arrow_left_right_circle". Available on cupertino_icons package 1.0.0+ only.
    pub fn arrow_left_right_circle() -> IconData {
        cupertino(0xf501)
    }

    /// Cupertino icon named "arrow_left_right_circle_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn arrow_left_right_circle_fill() -> IconData {
        cupertino(0xf502)
    }

    /// Cupertino icon named "arrow_left_right_square". Available on cupertino_icons package 1.0.0+ only.
    pub fn arrow_left_right_square() -> IconData {
        cupertino(0xf503)
    }

    /// Cupertino icon named "arrow_left_right_square_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn arrow_left_right_square_fill() -> IconData {
        cupertino(0xf504)
    }

    /// Cupertino icon named "arrow_left_square". Available on cupertino_icons package 1.0.0+ only.
    pub fn arrow_left_square() -> IconData {
        cupertino(0xf505)
    }

    /// Cupertino icon named "arrow_left_square_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn arrow_left_square_fill() -> IconData {
        cupertino(0xf506)
    }

    /// Cupertino icon named "arrow_left_to_line". Available on cupertino_icons package 1.0.0+ only.
    pub fn arrow_left_to_line() -> IconData {
        cupertino(0xf507)
    }

    /// Cupertino icon named "arrow_left_to_line_alt". Available on cupertino_icons package 1.0.0+ only.
    pub fn arrow_left_to_line_alt() -> IconData {
        cupertino(0xf508)
    }

    /// Cupertino icon named "arrow_merge". Available on cupertino_icons package 1.0.0+ only.
    pub fn arrow_merge() -> IconData {
        cupertino(0xf509)
    }

    /// Cupertino icon named "arrow_right". Available on cupertino_icons package 1.0.0+ only.
    pub fn arrow_right() -> IconData {
        cupertino(0xf50a)
    }

    /// Cupertino icon named "arrow_right_arrow_left". Available on cupertino_icons package 1.0.0+ only.
    pub fn arrow_right_arrow_left() -> IconData {
        cupertino(0xf50b)
    }

    /// Cupertino icon named "arrow_right_arrow_left_circle". Available on cupertino_icons package 1.0.0+ only.
    pub fn arrow_right_arrow_left_circle() -> IconData {
        cupertino(0xf50c)
    }

    /// Cupertino icon named "arrow_right_arrow_left_circle_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn arrow_right_arrow_left_circle_fill() -> IconData {
        cupertino(0xf50d)
    }

    /// Cupertino icon named "arrow_right_arrow_left_square". Available on cupertino_icons package 1.0.0+ only.
    pub fn arrow_right_arrow_left_square() -> IconData {
        cupertino(0xf50e)
    }

    /// Cupertino icon named "arrow_right_arrow_left_square_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn arrow_right_arrow_left_square_fill() -> IconData {
        cupertino(0xf50f)
    }

    /// Cupertino icon named "arrow_right_circle". Available on cupertino_icons package 1.0.0+ only.
    pub fn arrow_right_circle() -> IconData {
        cupertino(0xf510)
    }

    /// Cupertino icon named "arrow_right_circle_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn arrow_right_circle_fill() -> IconData {
        cupertino(0xf511)
    }

    /// Cupertino icon named "arrow_right_square". Available on cupertino_icons package 1.0.0+ only.
    pub fn arrow_right_square() -> IconData {
        cupertino(0xf512)
    }

    /// Cupertino icon named "arrow_right_square_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn arrow_right_square_fill() -> IconData {
        cupertino(0xf513)
    }

    /// Cupertino icon named "arrow_right_to_line". Available on cupertino_icons package 1.0.0+ only.
    pub fn arrow_right_to_line() -> IconData {
        cupertino(0xf514)
    }

    /// Cupertino icon named "arrow_right_to_line_alt". Available on cupertino_icons package 1.0.0+ only.
    pub fn arrow_right_to_line_alt() -> IconData {
        cupertino(0xf515)
    }

    /// Cupertino icon named "arrow_swap". Available on cupertino_icons package 1.0.0+ only.
    pub fn arrow_swap() -> IconData {
        cupertino(0xf516)
    }

    /// Cupertino icon named "arrow_turn_down_left". Available on cupertino_icons package 1.0.0+ only.
    pub fn arrow_turn_down_left() -> IconData {
        cupertino(0xf517)
    }

    /// Cupertino icon named "arrow_turn_down_right". Available on cupertino_icons package 1.0.0+ only.
    pub fn arrow_turn_down_right() -> IconData {
        cupertino(0xf518)
    }

    /// Cupertino icon named "arrow_turn_left_down". Available on cupertino_icons package 1.0.0+ only.
    pub fn arrow_turn_left_down() -> IconData {
        cupertino(0xf519)
    }

    /// Cupertino icon named "arrow_turn_left_up". Available on cupertino_icons package 1.0.0+ only.
    pub fn arrow_turn_left_up() -> IconData {
        cupertino(0xf51a)
    }

    /// Cupertino icon named "arrow_turn_right_down". Available on cupertino_icons package 1.0.0+ only.
    pub fn arrow_turn_right_down() -> IconData {
        cupertino(0xf51b)
    }

    /// Cupertino icon named "arrow_turn_right_up". Available on cupertino_icons package 1.0.0+ only.
    pub fn arrow_turn_right_up() -> IconData {
        cupertino(0xf51c)
    }

    /// Cupertino icon named "arrow_turn_up_left". Available on cupertino_icons package 1.0.0+ only.
    pub fn arrow_turn_up_left() -> IconData {
        cupertino(0xf51d)
    }

    /// Cupertino icon named "arrow_turn_up_right". Available on cupertino_icons package 1.0.0+ only.
    pub fn arrow_turn_up_right() -> IconData {
        cupertino(0xf51e)
    }

    /// Cupertino icon named "arrow_up". Available on cupertino_icons package 1.0.0+ only.
    /// This is the same icon as [`up_arrow`](Self::up_arrow()) which is available in cupertino_icons 0.1.3.
    pub fn arrow_up() -> IconData {
        cupertino(0xf366)
    }

    /// Cupertino icon named "arrow_up_arrow_down". Available on cupertino_icons package 1.0.0+ only.
    pub fn arrow_up_arrow_down() -> IconData {
        cupertino(0xf51f)
    }

    /// Cupertino icon named "arrow_up_arrow_down_circle". Available on cupertino_icons package 1.0.0+ only.
    pub fn arrow_up_arrow_down_circle() -> IconData {
        cupertino(0xf520)
    }

    /// Cupertino icon named "arrow_up_arrow_down_circle_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn arrow_up_arrow_down_circle_fill() -> IconData {
        cupertino(0xf521)
    }

    /// Cupertino icon named "arrow_up_arrow_down_square". Available on cupertino_icons package 1.0.0+ only.
    pub fn arrow_up_arrow_down_square() -> IconData {
        cupertino(0xf522)
    }

    /// Cupertino icon named "arrow_up_arrow_down_square_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn arrow_up_arrow_down_square_fill() -> IconData {
        cupertino(0xf523)
    }

    /// Cupertino icon named "arrow_up_bin". Available on cupertino_icons package 1.0.0+ only.
    pub fn arrow_up_bin() -> IconData {
        cupertino(0xf524)
    }

    /// Cupertino icon named "arrow_up_bin_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn arrow_up_bin_fill() -> IconData {
        cupertino(0xf525)
    }

    /// Cupertino icon named "arrow_up_circle". Available on cupertino_icons package 1.0.0+ only.
    pub fn arrow_up_circle() -> IconData {
        cupertino(0xf526)
    }

    /// Cupertino icon named "arrow_up_circle_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn arrow_up_circle_fill() -> IconData {
        cupertino(0xf527)
    }

    /// Cupertino icon named "arrow_up_doc". Available on cupertino_icons package 1.0.0+ only.
    pub fn arrow_up_doc() -> IconData {
        cupertino(0xf528)
    }

    /// Cupertino icon named "arrow_up_doc_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn arrow_up_doc_fill() -> IconData {
        cupertino(0xf529)
    }

    /// Cupertino icon named "arrow_up_down". Available on cupertino_icons package 1.0.0+ only.
    pub fn arrow_up_down() -> IconData {
        cupertino(0xf52a)
    }

    /// Cupertino icon named "arrow_up_down_circle". Available on cupertino_icons package 1.0.0+ only.
    pub fn arrow_up_down_circle() -> IconData {
        cupertino(0xf52b)
    }

    /// Cupertino icon named "arrow_up_down_circle_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn arrow_up_down_circle_fill() -> IconData {
        cupertino(0xf52c)
    }

    /// Cupertino icon named "arrow_up_down_square". Available on cupertino_icons package 1.0.0+ only.
    pub fn arrow_up_down_square() -> IconData {
        cupertino(0xf52d)
    }

    /// Cupertino icon named "arrow_up_down_square_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn arrow_up_down_square_fill() -> IconData {
        cupertino(0xf52e)
    }

    /// Cupertino icon named "arrow_up_left". Available on cupertino_icons package 1.0.0+ only.
    pub fn arrow_up_left() -> IconData {
        cupertino(0xf52f)
    }

    /// Cupertino icon named "arrow_up_left_arrow_down_right". Available on cupertino_icons package 1.0.0+ only.
    /// This is the same icon as [`fullscreen`](Self::fullscreen()) which is available in cupertino_icons 0.1.3.
    pub fn arrow_up_left_arrow_down_right() -> IconData {
        cupertino(0xf386)
    }

    /// Cupertino icon named "arrow_up_left_circle". Available on cupertino_icons package 1.0.0+ only.
    pub fn arrow_up_left_circle() -> IconData {
        cupertino(0xf530)
    }

    /// Cupertino icon named "arrow_up_left_circle_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn arrow_up_left_circle_fill() -> IconData {
        cupertino(0xf531)
    }

    /// Cupertino icon named "arrow_up_left_square". Available on cupertino_icons package 1.0.0+ only.
    pub fn arrow_up_left_square() -> IconData {
        cupertino(0xf532)
    }

    /// Cupertino icon named "arrow_up_left_square_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn arrow_up_left_square_fill() -> IconData {
        cupertino(0xf533)
    }

    /// Cupertino icon named "arrow_up_right". Available on cupertino_icons package 1.0.0+ only.
    pub fn arrow_up_right() -> IconData {
        cupertino(0xf534)
    }

    /// Cupertino icon named "arrow_up_right_circle". Available on cupertino_icons package 1.0.0+ only.
    pub fn arrow_up_right_circle() -> IconData {
        cupertino(0xf535)
    }

    /// Cupertino icon named "arrow_up_right_circle_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn arrow_up_right_circle_fill() -> IconData {
        cupertino(0xf536)
    }

    /// Cupertino icon named "arrow_up_right_diamond". Available on cupertino_icons package 1.0.0+ only.
    pub fn arrow_up_right_diamond() -> IconData {
        cupertino(0xf537)
    }

    /// Cupertino icon named "arrow_up_right_diamond_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn arrow_up_right_diamond_fill() -> IconData {
        cupertino(0xf538)
    }

    /// Cupertino icon named "arrow_up_right_square". Available on cupertino_icons package 1.0.0+ only.
    pub fn arrow_up_right_square() -> IconData {
        cupertino(0xf539)
    }

    /// Cupertino icon named "arrow_up_right_square_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn arrow_up_right_square_fill() -> IconData {
        cupertino(0xf53a)
    }

    /// Cupertino icon named "arrow_up_square". Available on cupertino_icons package 1.0.0+ only.
    pub fn arrow_up_square() -> IconData {
        cupertino(0xf53b)
    }

    /// Cupertino icon named "arrow_up_square_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn arrow_up_square_fill() -> IconData {
        cupertino(0xf53c)
    }

    /// Cupertino icon named "arrow_up_to_line". Available on cupertino_icons package 1.0.0+ only.
    pub fn arrow_up_to_line() -> IconData {
        cupertino(0xf53d)
    }

    /// Cupertino icon named "arrow_up_to_line_alt". Available on cupertino_icons package 1.0.0+ only.
    pub fn arrow_up_to_line_alt() -> IconData {
        cupertino(0xf53e)
    }

    /// Cupertino icon named "arrow_uturn_down". Available on cupertino_icons package 1.0.0+ only.
    pub fn arrow_uturn_down() -> IconData {
        cupertino(0xf53f)
    }

    /// Cupertino icon named "arrow_uturn_down_circle". Available on cupertino_icons package 1.0.0+ only.
    pub fn arrow_uturn_down_circle() -> IconData {
        cupertino(0xf540)
    }

    /// Cupertino icon named "arrow_uturn_down_circle_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn arrow_uturn_down_circle_fill() -> IconData {
        cupertino(0xf541)
    }

    /// Cupertino icon named "arrow_uturn_down_square". Available on cupertino_icons package 1.0.0+ only.
    pub fn arrow_uturn_down_square() -> IconData {
        cupertino(0xf542)
    }

    /// Cupertino icon named "arrow_uturn_down_square_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn arrow_uturn_down_square_fill() -> IconData {
        cupertino(0xf543)
    }

    /// Cupertino icon named "arrow_uturn_left". Available on cupertino_icons package 1.0.0+ only.
    pub fn arrow_uturn_left() -> IconData {
        cupertino(0xf544)
    }

    /// Cupertino icon named "arrow_uturn_left_circle". Available on cupertino_icons package 1.0.0+ only.
    pub fn arrow_uturn_left_circle() -> IconData {
        cupertino(0xf545)
    }

    /// Cupertino icon named "arrow_uturn_left_circle_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn arrow_uturn_left_circle_fill() -> IconData {
        cupertino(0xf546)
    }

    /// Cupertino icon named "arrow_uturn_left_square". Available on cupertino_icons package 1.0.0+ only.
    pub fn arrow_uturn_left_square() -> IconData {
        cupertino(0xf547)
    }

    /// Cupertino icon named "arrow_uturn_left_square_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn arrow_uturn_left_square_fill() -> IconData {
        cupertino(0xf548)
    }

    /// Cupertino icon named "arrow_uturn_right". Available on cupertino_icons package 1.0.0+ only.
    pub fn arrow_uturn_right() -> IconData {
        cupertino(0xf549)
    }

    /// Cupertino icon named "arrow_uturn_right_circle". Available on cupertino_icons package 1.0.0+ only.
    pub fn arrow_uturn_right_circle() -> IconData {
        cupertino(0xf54a)
    }

    /// Cupertino icon named "arrow_uturn_right_circle_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn arrow_uturn_right_circle_fill() -> IconData {
        cupertino(0xf54b)
    }

    /// Cupertino icon named "arrow_uturn_right_square". Available on cupertino_icons package 1.0.0+ only.
    pub fn arrow_uturn_right_square() -> IconData {
        cupertino(0xf54c)
    }

    /// Cupertino icon named "arrow_uturn_right_square_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn arrow_uturn_right_square_fill() -> IconData {
        cupertino(0xf54d)
    }

    /// Cupertino icon named "arrow_uturn_up". Available on cupertino_icons package 1.0.0+ only.
    pub fn arrow_uturn_up() -> IconData {
        cupertino(0xf54e)
    }

    /// Cupertino icon named "arrow_uturn_up_circle". Available on cupertino_icons package 1.0.0+ only.
    pub fn arrow_uturn_up_circle() -> IconData {
        cupertino(0xf54f)
    }

    /// Cupertino icon named "arrow_uturn_up_circle_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn arrow_uturn_up_circle_fill() -> IconData {
        cupertino(0xf550)
    }

    /// Cupertino icon named "arrow_uturn_up_square". Available on cupertino_icons package 1.0.0+ only.
    pub fn arrow_uturn_up_square() -> IconData {
        cupertino(0xf551)
    }

    /// Cupertino icon named "arrow_uturn_up_square_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn arrow_uturn_up_square_fill() -> IconData {
        cupertino(0xf552)
    }

    /// Cupertino icon named "arrowshape_turn_up_left". Available on cupertino_icons package 1.0.0+ only.
    /// This is the same icon as [`reply`](Self::reply()) which is available in cupertino_icons 0.1.3.
    pub fn arrowshape_turn_up_left() -> IconData {
        cupertino(0xf4c6)
    }

    /// Cupertino icon named "arrowshape_turn_up_left_2". Available on cupertino_icons package 1.0.0+ only.
    /// This is the same icon as [`reply_all`](Self::reply_all()) which is available in cupertino_icons 0.1.3.
    pub fn arrowshape_turn_up_left_2() -> IconData {
        cupertino(0xf21d)
    }

    /// Cupertino icon named "arrowshape_turn_up_left_2_fill". Available on cupertino_icons package 1.0.0+ only.
    /// This is the same icon as [`reply_thick_solid`](Self::reply_thick_solid()) which is available in cupertino_icons 0.1.3.
    pub fn arrowshape_turn_up_left_2_fill() -> IconData {
        cupertino(0xf21e)
    }

    /// Cupertino icon named "arrowshape_turn_up_left_circle". Available on cupertino_icons package 1.0.0+ only.
    pub fn arrowshape_turn_up_left_circle() -> IconData {
        cupertino(0xf553)
    }

    /// Cupertino icon named "arrowshape_turn_up_left_circle_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn arrowshape_turn_up_left_circle_fill() -> IconData {
        cupertino(0xf554)
    }

    /// Cupertino icon named "arrowshape_turn_up_left_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn arrowshape_turn_up_left_fill() -> IconData {
        cupertino(0xf555)
    }

    /// Cupertino icon named "arrowshape_turn_up_right". Available on cupertino_icons package 1.0.0+ only.
    pub fn arrowshape_turn_up_right() -> IconData {
        cupertino(0xf556)
    }

    /// Cupertino icon named "arrowshape_turn_up_right_circle". Available on cupertino_icons package 1.0.0+ only.
    pub fn arrowshape_turn_up_right_circle() -> IconData {
        cupertino(0xf557)
    }

    /// Cupertino icon named "arrowshape_turn_up_right_circle_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn arrowshape_turn_up_right_circle_fill() -> IconData {
        cupertino(0xf558)
    }

    /// Cupertino icon named "arrowshape_turn_up_right_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn arrowshape_turn_up_right_fill() -> IconData {
        cupertino(0xf559)
    }

    /// Cupertino icon named "arrowtriangle_down". Available on cupertino_icons package 1.0.0+ only.
    pub fn arrowtriangle_down() -> IconData {
        cupertino(0xf55a)
    }

    /// Cupertino icon named "arrowtriangle_down_circle". Available on cupertino_icons package 1.0.0+ only.
    pub fn arrowtriangle_down_circle() -> IconData {
        cupertino(0xf55b)
    }

    /// Cupertino icon named "arrowtriangle_down_circle_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn arrowtriangle_down_circle_fill() -> IconData {
        cupertino(0xf55c)
    }

    /// Cupertino icon named "arrowtriangle_down_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn arrowtriangle_down_fill() -> IconData {
        cupertino(0xf55d)
    }

    /// Cupertino icon named "arrowtriangle_down_square". Available on cupertino_icons package 1.0.0+ only.
    pub fn arrowtriangle_down_square() -> IconData {
        cupertino(0xf55e)
    }

    /// Cupertino icon named "arrowtriangle_down_square_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn arrowtriangle_down_square_fill() -> IconData {
        cupertino(0xf55f)
    }

    /// Cupertino icon named "arrowtriangle_left". Available on cupertino_icons package 1.0.0+ only.
    pub fn arrowtriangle_left() -> IconData {
        cupertino(0xf560)
    }

    /// Cupertino icon named "arrowtriangle_left_circle". Available on cupertino_icons package 1.0.0+ only.
    pub fn arrowtriangle_left_circle() -> IconData {
        cupertino(0xf561)
    }

    /// Cupertino icon named "arrowtriangle_left_circle_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn arrowtriangle_left_circle_fill() -> IconData {
        cupertino(0xf562)
    }

    /// Cupertino icon named "arrowtriangle_left_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn arrowtriangle_left_fill() -> IconData {
        cupertino(0xf563)
    }

    /// Cupertino icon named "arrowtriangle_left_square". Available on cupertino_icons package 1.0.0+ only.
    pub fn arrowtriangle_left_square() -> IconData {
        cupertino(0xf564)
    }

    /// Cupertino icon named "arrowtriangle_left_square_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn arrowtriangle_left_square_fill() -> IconData {
        cupertino(0xf565)
    }

    /// Cupertino icon named "arrowtriangle_right". Available on cupertino_icons package 1.0.0+ only.
    pub fn arrowtriangle_right() -> IconData {
        cupertino(0xf566)
    }

    /// Cupertino icon named "arrowtriangle_right_circle". Available on cupertino_icons package 1.0.0+ only.
    pub fn arrowtriangle_right_circle() -> IconData {
        cupertino(0xf567)
    }

    /// Cupertino icon named "arrowtriangle_right_circle_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn arrowtriangle_right_circle_fill() -> IconData {
        cupertino(0xf568)
    }

    /// Cupertino icon named "arrowtriangle_right_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn arrowtriangle_right_fill() -> IconData {
        cupertino(0xf569)
    }

    /// Cupertino icon named "arrowtriangle_right_square". Available on cupertino_icons package 1.0.0+ only.
    pub fn arrowtriangle_right_square() -> IconData {
        cupertino(0xf56a)
    }

    /// Cupertino icon named "arrowtriangle_right_square_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn arrowtriangle_right_square_fill() -> IconData {
        cupertino(0xf56b)
    }

    /// Cupertino icon named "arrowtriangle_up". Available on cupertino_icons package 1.0.0+ only.
    pub fn arrowtriangle_up() -> IconData {
        cupertino(0xf56c)
    }

    /// Cupertino icon named "arrowtriangle_up_circle". Available on cupertino_icons package 1.0.0+ only.
    pub fn arrowtriangle_up_circle() -> IconData {
        cupertino(0xf56d)
    }

    /// Cupertino icon named "arrowtriangle_up_circle_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn arrowtriangle_up_circle_fill() -> IconData {
        cupertino(0xf56e)
    }

    /// Cupertino icon named "arrowtriangle_up_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn arrowtriangle_up_fill() -> IconData {
        cupertino(0xf56f)
    }

    /// Cupertino icon named "arrowtriangle_up_square". Available on cupertino_icons package 1.0.0+ only.
    pub fn arrowtriangle_up_square() -> IconData {
        cupertino(0xf570)
    }

    /// Cupertino icon named "arrowtriangle_up_square_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn arrowtriangle_up_square_fill() -> IconData {
        cupertino(0xf571)
    }

    /// Cupertino icon named "asterisk_circle". Available on cupertino_icons package 1.0.0+ only.
    pub fn asterisk_circle() -> IconData {
        cupertino(0xf572)
    }

    /// Cupertino icon named "asterisk_circle_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn asterisk_circle_fill() -> IconData {
        cupertino(0xf573)
    }

    /// Cupertino icon named "at". Available on cupertino_icons package 1.0.0+ only.
    pub fn at() -> IconData {
        cupertino(0xf574)
    }

    /// Cupertino icon named "at_badge_minus". Available on cupertino_icons package 1.0.0+ only.
    pub fn at_badge_minus() -> IconData {
        cupertino(0xf575)
    }

    /// Cupertino icon named "at_badge_plus". Available on cupertino_icons package 1.0.0+ only.
    pub fn at_badge_plus() -> IconData {
        cupertino(0xf576)
    }

    /// Cupertino icon named "at_circle". Available on cupertino_icons package 1.0.0+ only.
    pub fn at_circle() -> IconData {
        cupertino(0xf8af)
    }

    /// Cupertino icon named "at_circle_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn at_circle_fill() -> IconData {
        cupertino(0xf8b0)
    }

    /// Cupertino icon named "backward". Available on cupertino_icons package 1.0.0+ only.
    pub fn backward() -> IconData {
        cupertino(0xf577)
    }

    /// Cupertino icon named "backward_end". Available on cupertino_icons package 1.0.0+ only.
    pub fn backward_end() -> IconData {
        cupertino(0xf578)
    }

    /// Cupertino icon named "backward_end_alt". Available on cupertino_icons package 1.0.0+ only.
    pub fn backward_end_alt() -> IconData {
        cupertino(0xf579)
    }

    /// Cupertino icon named "backward_end_alt_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn backward_end_alt_fill() -> IconData {
        cupertino(0xf57a)
    }

    /// Cupertino icon named "backward_end_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn backward_end_fill() -> IconData {
        cupertino(0xf57b)
    }

    /// Cupertino icon named "backward_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn backward_fill() -> IconData {
        cupertino(0xf57c)
    }

    /// Cupertino icon named "badge_plus_radiowaves_right". Available on cupertino_icons package 1.0.0+ only.
    pub fn badge_plus_radiowaves_right() -> IconData {
        cupertino(0xf57d)
    }

    /// Cupertino icon named "bag". Available on cupertino_icons package 1.0.0+ only.
    pub fn bag() -> IconData {
        cupertino(0xf57e)
    }

    /// Cupertino icon named "bag_badge_minus". Available on cupertino_icons package 1.0.0+ only.
    pub fn bag_badge_minus() -> IconData {
        cupertino(0xf57f)
    }

    /// Cupertino icon named "bag_badge_plus". Available on cupertino_icons package 1.0.0+ only.
    pub fn bag_badge_plus() -> IconData {
        cupertino(0xf580)
    }

    /// Cupertino icon named "bag_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn bag_fill() -> IconData {
        cupertino(0xf581)
    }

    /// Cupertino icon named "bag_fill_badge_minus". Available on cupertino_icons package 1.0.0+ only.
    pub fn bag_fill_badge_minus() -> IconData {
        cupertino(0xf582)
    }

    /// Cupertino icon named "bag_fill_badge_plus". Available on cupertino_icons package 1.0.0+ only.
    pub fn bag_fill_badge_plus() -> IconData {
        cupertino(0xf583)
    }

    /// Cupertino icon named "bandage". Available on cupertino_icons package 1.0.0+ only.
    pub fn bandage() -> IconData {
        cupertino(0xf584)
    }

    /// Cupertino icon named "bandage_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn bandage_fill() -> IconData {
        cupertino(0xf585)
    }

    /// Cupertino icon named "barcode". Available on cupertino_icons package 1.0.0+ only.
    pub fn barcode() -> IconData {
        cupertino(0xf586)
    }

    /// Cupertino icon named "barcode_viewfinder". Available on cupertino_icons package 1.0.0+ only.
    pub fn barcode_viewfinder() -> IconData {
        cupertino(0xf587)
    }

    /// Cupertino icon named "bars". Available on cupertino_icons package 1.0.0+ only.
    pub fn bars() -> IconData {
        cupertino(0xf8b1)
    }

    /// Cupertino icon named "battery_0". Available on cupertino_icons package 1.0.0+ only.
    /// This is the same icon as [`battery_empty`](Self::battery_empty()) which is available in cupertino_icons 0.1.3.
    pub fn battery_0() -> IconData {
        cupertino(0xf112)
    }

    /// Cupertino icon named "battery_100". Available on cupertino_icons package 1.0.0+ only.
    /// This is the same icon as [`battery_charging`](Self::battery_charging()) which is available in cupertino_icons 0.1.3.
    /// This is the same icon as [`battery_full`](Self::battery_full()) which is available in cupertino_icons 0.1.3.
    /// This is the same icon as [`battery_75_percent`](Self::battery_75_percent()) which is available in cupertino_icons 0.1.3.
    pub fn battery_100() -> IconData {
        cupertino(0xf113)
    }

    /// Cupertino icon named "battery_25". Available on cupertino_icons package 1.0.0+ only.
    /// This is the same icon as [`battery_25_percent`](Self::battery_25_percent()) which is available in cupertino_icons 0.1.3.
    pub fn battery_25() -> IconData {
        cupertino(0xf115)
    }

    /// Cupertino icon named "bed_double". Available on cupertino_icons package 1.0.0+ only.
    pub fn bed_double() -> IconData {
        cupertino(0xf588)
    }

    /// Cupertino icon named "bed_double_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn bed_double_fill() -> IconData {
        cupertino(0xf589)
    }

    /// Cupertino icon named "bell_circle". Available on cupertino_icons package 1.0.0+ only.
    pub fn bell_circle() -> IconData {
        cupertino(0xf58a)
    }

    /// Cupertino icon named "bell_circle_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn bell_circle_fill() -> IconData {
        cupertino(0xf58b)
    }

    /// Cupertino icon named "bell_fill". Available on cupertino_icons package 1.0.0+ only.
    /// This is the same icon as [`bell_solid`](Self::bell_solid()) which is available in cupertino_icons 0.1.3.
    pub fn bell_fill() -> IconData {
        cupertino(0xf3e2)
    }

    /// Cupertino icon named "bell_slash". Available on cupertino_icons package 1.0.0+ only.
    pub fn bell_slash() -> IconData {
        cupertino(0xf58c)
    }

    /// Cupertino icon named "bell_slash_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn bell_slash_fill() -> IconData {
        cupertino(0xf58d)
    }

    /// Cupertino icon named "bin_xmark". Available on cupertino_icons package 1.0.0+ only.
    pub fn bin_xmark() -> IconData {
        cupertino(0xf58e)
    }

    /// Cupertino icon named "bin_xmark_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn bin_xmark_fill() -> IconData {
        cupertino(0xf58f)
    }

    /// Cupertino icon named "bitcoin". Available on cupertino_icons package 1.0.0+ only.
    pub fn bitcoin() -> IconData {
        cupertino(0xf8b2)
    }

    /// Cupertino icon named "bitcoin_circle". Available on cupertino_icons package 1.0.0+ only.
    pub fn bitcoin_circle() -> IconData {
        cupertino(0xf8b3)
    }

    /// Cupertino icon named "bitcoin_circle_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn bitcoin_circle_fill() -> IconData {
        cupertino(0xf8b4)
    }

    /// Cupertino icon named "bold". Available on cupertino_icons package 1.0.0+ only.
    pub fn bold() -> IconData {
        cupertino(0xf590)
    }

    /// Cupertino icon named "bold_italic_underline". Available on cupertino_icons package 1.0.0+ only.
    pub fn bold_italic_underline() -> IconData {
        cupertino(0xf591)
    }

    /// Cupertino icon named "bold_underline". Available on cupertino_icons package 1.0.0+ only.
    pub fn bold_underline() -> IconData {
        cupertino(0xf592)
    }

    /// Cupertino icon named "bolt". Available on cupertino_icons package 1.0.0+ only.
    pub fn bolt() -> IconData {
        cupertino(0xf593)
    }

    /// Cupertino icon named "bolt_badge_a". Available on cupertino_icons package 1.0.0+ only.
    pub fn bolt_badge_a() -> IconData {
        cupertino(0xf594)
    }

    /// Cupertino icon named "bolt_badge_a_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn bolt_badge_a_fill() -> IconData {
        cupertino(0xf595)
    }

    /// Cupertino icon named "bolt_circle". Available on cupertino_icons package 1.0.0+ only.
    pub fn bolt_circle() -> IconData {
        cupertino(0xf596)
    }

    /// Cupertino icon named "bolt_circle_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn bolt_circle_fill() -> IconData {
        cupertino(0xf597)
    }

    /// Cupertino icon named "bolt_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn bolt_fill() -> IconData {
        cupertino(0xf598)
    }

    /// Cupertino icon named "bolt_horizontal". Available on cupertino_icons package 1.0.0+ only.
    pub fn bolt_horizontal() -> IconData {
        cupertino(0xf599)
    }

    /// Cupertino icon named "bolt_horizontal_circle". Available on cupertino_icons package 1.0.0+ only.
    pub fn bolt_horizontal_circle() -> IconData {
        cupertino(0xf59a)
    }

    /// Cupertino icon named "bolt_horizontal_circle_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn bolt_horizontal_circle_fill() -> IconData {
        cupertino(0xf59b)
    }

    /// Cupertino icon named "bolt_horizontal_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn bolt_horizontal_fill() -> IconData {
        cupertino(0xf59c)
    }

    /// Cupertino icon named "bolt_slash". Available on cupertino_icons package 1.0.0+ only.
    pub fn bolt_slash() -> IconData {
        cupertino(0xf59d)
    }

    /// Cupertino icon named "bolt_slash_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn bolt_slash_fill() -> IconData {
        cupertino(0xf59e)
    }

    /// Cupertino icon named "book_circle". Available on cupertino_icons package 1.0.0+ only.
    pub fn book_circle() -> IconData {
        cupertino(0xf59f)
    }

    /// Cupertino icon named "book_circle_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn book_circle_fill() -> IconData {
        cupertino(0xf5a0)
    }

    /// Cupertino icon named "book_fill". Available on cupertino_icons package 1.0.0+ only.
    /// This is the same icon as [`book_solid`](Self::book_solid()) which is available in cupertino_icons 0.1.3.
    pub fn book_fill() -> IconData {
        cupertino(0xf3e8)
    }

    /// Cupertino icon named "bookmark_fill". Available on cupertino_icons package 1.0.0+ only.
    /// This is the same icon as [`bookmark_solid`](Self::bookmark_solid()) which is available in cupertino_icons 0.1.3.
    pub fn bookmark_fill() -> IconData {
        cupertino(0xf3ea)
    }

    /// Cupertino icon named "briefcase". Available on cupertino_icons package 1.0.0+ only.
    pub fn briefcase() -> IconData {
        cupertino(0xf5a1)
    }

    /// Cupertino icon named "briefcase_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn briefcase_fill() -> IconData {
        cupertino(0xf5a2)
    }

    /// Cupertino icon named "bubble_left". Available on cupertino_icons package 1.0.0+ only.
    pub fn bubble_left() -> IconData {
        cupertino(0xf5a3)
    }

    /// Cupertino icon named "bubble_left_bubble_right". Available on cupertino_icons package 1.0.0+ only.
    pub fn bubble_left_bubble_right() -> IconData {
        cupertino(0xf5a4)
    }

    /// Cupertino icon named "bubble_left_bubble_right_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn bubble_left_bubble_right_fill() -> IconData {
        cupertino(0xf5a5)
    }

    /// Cupertino icon named "bubble_left_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn bubble_left_fill() -> IconData {
        cupertino(0xf5a6)
    }

    /// Cupertino icon named "bubble_middle_bottom". Available on cupertino_icons package 1.0.0+ only.
    pub fn bubble_middle_bottom() -> IconData {
        cupertino(0xf5a7)
    }

    /// Cupertino icon named "bubble_middle_bottom_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn bubble_middle_bottom_fill() -> IconData {
        cupertino(0xf5a8)
    }

    /// Cupertino icon named "bubble_middle_top". Available on cupertino_icons package 1.0.0+ only.
    pub fn bubble_middle_top() -> IconData {
        cupertino(0xf5a9)
    }

    /// Cupertino icon named "bubble_middle_top_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn bubble_middle_top_fill() -> IconData {
        cupertino(0xf5aa)
    }

    /// Cupertino icon named "bubble_right". Available on cupertino_icons package 1.0.0+ only.
    pub fn bubble_right() -> IconData {
        cupertino(0xf5ab)
    }

    /// Cupertino icon named "bubble_right_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn bubble_right_fill() -> IconData {
        cupertino(0xf5ac)
    }

    /// Cupertino icon named "building_2_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn building_2_fill() -> IconData {
        cupertino(0xf8b5)
    }

    /// Cupertino icon named "burn". Available on cupertino_icons package 1.0.0+ only.
    pub fn burn() -> IconData {
        cupertino(0xf5ad)
    }

    /// Cupertino icon named "burst". Available on cupertino_icons package 1.0.0+ only.
    pub fn burst() -> IconData {
        cupertino(0xf5ae)
    }

    /// Cupertino icon named "burst_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn burst_fill() -> IconData {
        cupertino(0xf5af)
    }

    /// Cupertino icon named "calendar". Available on cupertino_icons package 1.0.0+ only.
    pub fn calendar() -> IconData {
        cupertino(0xf5b0)
    }

    /// Cupertino icon named "calendar_badge_minus". Available on cupertino_icons package 1.0.0+ only.
    pub fn calendar_badge_minus() -> IconData {
        cupertino(0xf5b1)
    }

    /// Cupertino icon named "calendar_badge_plus". Available on cupertino_icons package 1.0.0+ only.
    pub fn calendar_badge_plus() -> IconData {
        cupertino(0xf5b2)
    }

    /// Cupertino icon named "calendar_circle". Available on cupertino_icons package 1.0.0+ only.
    pub fn calendar_circle() -> IconData {
        cupertino(0xf5b3)
    }

    /// Cupertino icon named "calendar_circle_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn calendar_circle_fill() -> IconData {
        cupertino(0xf5b4)
    }

    /// Cupertino icon named "calendar_today". Available on cupertino_icons package 1.0.0+ only.
    pub fn calendar_today() -> IconData {
        cupertino(0xf8b6)
    }

    /// Cupertino icon named "camera". Available on cupertino_icons package 1.0.0+ only.
    /// This is the same icon as [`photo_camera`](Self::photo_camera()) which is available in cupertino_icons 0.1.3.
    pub fn camera() -> IconData {
        cupertino(0xf3f5)
    }

    /// Cupertino icon named "camera_circle". Available on cupertino_icons package 1.0.0+ only.
    pub fn camera_circle() -> IconData {
        cupertino(0xf5b5)
    }

    /// Cupertino icon named "camera_circle_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn camera_circle_fill() -> IconData {
        cupertino(0xf5b6)
    }

    /// Cupertino icon named "camera_fill". Available on cupertino_icons package 1.0.0+ only.
    /// This is the same icon as [`photo_camera_solid`](Self::photo_camera_solid()) which is available in cupertino_icons 0.1.3.
    pub fn camera_fill() -> IconData {
        cupertino(0xf3f6)
    }

    /// Cupertino icon named "camera_on_rectangle". Available on cupertino_icons package 1.0.0+ only.
    pub fn camera_on_rectangle() -> IconData {
        cupertino(0xf5b7)
    }

    /// Cupertino icon named "camera_on_rectangle_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn camera_on_rectangle_fill() -> IconData {
        cupertino(0xf5b8)
    }

    /// Cupertino icon named "camera_rotate". Available on cupertino_icons package 1.0.0+ only.
    /// This is the same icon as [`switch_camera`](Self::switch_camera()) which is available in cupertino_icons 0.1.3.
    pub fn camera_rotate() -> IconData {
        cupertino(0xf49e)
    }

    /// Cupertino icon named "camera_rotate_fill". Available on cupertino_icons package 1.0.0+ only.
    /// This is the same icon as [`switch_camera_solid`](Self::switch_camera_solid()) which is available in cupertino_icons 0.1.3.
    pub fn camera_rotate_fill() -> IconData {
        cupertino(0xf49f)
    }

    /// Cupertino icon named "camera_viewfinder". Available on cupertino_icons package 1.0.0+ only.
    pub fn camera_viewfinder() -> IconData {
        cupertino(0xf5b9)
    }

    /// Cupertino icon named "capslock". Available on cupertino_icons package 1.0.0+ only.
    pub fn capslock() -> IconData {
        cupertino(0xf5ba)
    }

    /// Cupertino icon named "capslock_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn capslock_fill() -> IconData {
        cupertino(0xf5bb)
    }

    /// Cupertino icon named "capsule". Available on cupertino_icons package 1.0.0+ only.
    pub fn capsule() -> IconData {
        cupertino(0xf5bc)
    }

    /// Cupertino icon named "capsule_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn capsule_fill() -> IconData {
        cupertino(0xf5bd)
    }

    /// Cupertino icon named "captions_bubble". Available on cupertino_icons package 1.0.0+ only.
    pub fn captions_bubble() -> IconData {
        cupertino(0xf5be)
    }

    /// Cupertino icon named "captions_bubble_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn captions_bubble_fill() -> IconData {
        cupertino(0xf5bf)
    }

    /// Cupertino icon named "car_fill". Available on cupertino_icons package 1.0.0+ only.
    /// This is the same icon as [`car`](Self::car()) which is available in cupertino_icons 0.1.3.
    pub fn car_fill() -> IconData {
        cupertino(0xf36f)
    }

    /// Cupertino icon named "cart". Available on cupertino_icons package 1.0.0+ only.
    /// This is the same icon as [`shopping_cart`](Self::shopping_cart()) which is available in cupertino_icons 0.1.3.
    pub fn cart() -> IconData {
        cupertino(0xf3f7)
    }

    /// Cupertino icon named "cart_badge_minus". Available on cupertino_icons package 1.0.0+ only.
    pub fn cart_badge_minus() -> IconData {
        cupertino(0xf5c0)
    }

    /// Cupertino icon named "cart_badge_plus". Available on cupertino_icons package 1.0.0+ only.
    pub fn cart_badge_plus() -> IconData {
        cupertino(0xf5c1)
    }

    /// Cupertino icon named "cart_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn cart_fill() -> IconData {
        cupertino(0xf5c2)
    }

    /// Cupertino icon named "cart_fill_badge_minus". Available on cupertino_icons package 1.0.0+ only.
    pub fn cart_fill_badge_minus() -> IconData {
        cupertino(0xf5c3)
    }

    /// Cupertino icon named "cart_fill_badge_plus". Available on cupertino_icons package 1.0.0+ only.
    pub fn cart_fill_badge_plus() -> IconData {
        cupertino(0xf5c4)
    }

    /// Cupertino icon named "chart_bar". Available on cupertino_icons package 1.0.0+ only.
    pub fn chart_bar() -> IconData {
        cupertino(0xf5c5)
    }

    /// Cupertino icon named "chart_bar_alt_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn chart_bar_alt_fill() -> IconData {
        cupertino(0xf8b7)
    }

    /// Cupertino icon named "chart_bar_circle". Available on cupertino_icons package 1.0.0+ only.
    pub fn chart_bar_circle() -> IconData {
        cupertino(0xf8b8)
    }

    /// Cupertino icon named "chart_bar_circle_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn chart_bar_circle_fill() -> IconData {
        cupertino(0xf8b9)
    }

    /// Cupertino icon named "chart_bar_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn chart_bar_fill() -> IconData {
        cupertino(0xf5c6)
    }

    /// Cupertino icon named "chart_bar_square". Available on cupertino_icons package 1.0.0+ only.
    pub fn chart_bar_square() -> IconData {
        cupertino(0xf8ba)
    }

    /// Cupertino icon named "chart_bar_square_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn chart_bar_square_fill() -> IconData {
        cupertino(0xf8bb)
    }

    /// Cupertino icon named "chart_pie". Available on cupertino_icons package 1.0.0+ only.
    pub fn chart_pie() -> IconData {
        cupertino(0xf5c7)
    }

    /// Cupertino icon named "chart_pie_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn chart_pie_fill() -> IconData {
        cupertino(0xf5c8)
    }

    /// Cupertino icon named "chat_bubble". Available on cupertino_icons package 1.0.0+ only.
    /// This is the same icon as [`conversation_bubble`](Self::conversation_bubble()) which is available in cupertino_icons 0.1.3.
    pub fn chat_bubble() -> IconData {
        cupertino(0xf3fb)
    }

    /// Cupertino icon named "chat_bubble_2". Available on cupertino_icons package 1.0.0+ only.
    pub fn chat_bubble_2() -> IconData {
        cupertino(0xf8bc)
    }

    /// Cupertino icon named "chat_bubble_2_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn chat_bubble_2_fill() -> IconData {
        cupertino(0xf8bd)
    }

    /// Cupertino icon named "chat_bubble_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn chat_bubble_fill() -> IconData {
        cupertino(0xf8be)
    }

    /// Cupertino icon named "chat_bubble_text". Available on cupertino_icons package 1.0.0+ only.
    pub fn chat_bubble_text() -> IconData {
        cupertino(0xf8bf)
    }

    /// Cupertino icon named "chat_bubble_text_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn chat_bubble_text_fill() -> IconData {
        cupertino(0xf8c0)
    }

    /// Cupertino icon named "checkmark". Available on cupertino_icons package 1.0.0+ only.
    /// This is the same icon as [`check_mark`](Self::check_mark()) which is available in cupertino_icons 0.1.3.
    pub fn checkmark() -> IconData {
        cupertino(0xf3fd)
    }

    /// Cupertino icon named "checkmark_alt". Available on cupertino_icons package 1.0.0+ only.
    pub fn checkmark_alt() -> IconData {
        cupertino(0xf8c1)
    }

    /// Cupertino icon named "checkmark_alt_circle". Available on cupertino_icons package 1.0.0+ only.
    pub fn checkmark_alt_circle() -> IconData {
        cupertino(0xf8c2)
    }

    /// Cupertino icon named "checkmark_alt_circle_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn checkmark_alt_circle_fill() -> IconData {
        cupertino(0xf8c3)
    }

    /// Cupertino icon named "checkmark_circle". Available on cupertino_icons package 1.0.0+ only.
    /// This is the same icon as [`check_mark_circled`](Self::check_mark_circled()) which is available in cupertino_icons 0.1.3.
    pub fn checkmark_circle() -> IconData {
        cupertino(0xf3fe)
    }

    /// Cupertino icon named "checkmark_circle_fill". Available on cupertino_icons package 1.0.0+ only.
    /// This is the same icon as [`check_mark_circled_solid`](Self::check_mark_circled_solid()) which is available in cupertino_icons 0.1.3.
    pub fn checkmark_circle_fill() -> IconData {
        cupertino(0xf3ff)
    }

    /// Cupertino icon named "checkmark_rectangle". Available on cupertino_icons package 1.0.0+ only.
    pub fn checkmark_rectangle() -> IconData {
        cupertino(0xf5c9)
    }

    /// Cupertino icon named "checkmark_rectangle_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn checkmark_rectangle_fill() -> IconData {
        cupertino(0xf5ca)
    }

    /// Cupertino icon named "checkmark_seal". Available on cupertino_icons package 1.0.0+ only.
    pub fn checkmark_seal() -> IconData {
        cupertino(0xf5cb)
    }

    /// Cupertino icon named "checkmark_seal_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn checkmark_seal_fill() -> IconData {
        cupertino(0xf5cc)
    }

    /// Cupertino icon named "checkmark_shield". Available on cupertino_icons package 1.0.0+ only.
    pub fn checkmark_shield() -> IconData {
        cupertino(0xf5cd)
    }

    /// Cupertino icon named "checkmark_shield_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn checkmark_shield_fill() -> IconData {
        cupertino(0xf5ce)
    }

    /// Cupertino icon named "checkmark_square". Available on cupertino_icons package 1.0.0+ only.
    pub fn checkmark_square() -> IconData {
        cupertino(0xf5cf)
    }

    /// Cupertino icon named "checkmark_square_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn checkmark_square_fill() -> IconData {
        cupertino(0xf5d0)
    }

    /// Cupertino icon named "chevron_back". Available on cupertino_icons package 1.0.0+ only.
    /// This is the same icon as [`back`](Self::back()) which is available in cupertino_icons 0.1.3.
    pub fn chevron_back() -> IconData {
        cupertino(0xf3cf)
    }

    /// Cupertino icon named "chevron_compact_down". Available on cupertino_icons package 1.0.0+ only.
    pub fn chevron_compact_down() -> IconData {
        cupertino(0xf5d1)
    }

    /// Cupertino icon named "chevron_compact_left". Available on cupertino_icons package 1.0.0+ only.
    pub fn chevron_compact_left() -> IconData {
        cupertino(0xf5d2)
    }

    /// Cupertino icon named "chevron_compact_right". Available on cupertino_icons package 1.0.0+ only.
    pub fn chevron_compact_right() -> IconData {
        cupertino(0xf5d3)
    }

    /// Cupertino icon named "chevron_compact_up". Available on cupertino_icons package 1.0.0+ only.
    pub fn chevron_compact_up() -> IconData {
        cupertino(0xf5d4)
    }

    /// Cupertino icon named "chevron_down". Available on cupertino_icons package 1.0.0+ only.
    pub fn chevron_down() -> IconData {
        cupertino(0xf5d5)
    }

    /// Cupertino icon named "chevron_down_circle". Available on cupertino_icons package 1.0.0+ only.
    pub fn chevron_down_circle() -> IconData {
        cupertino(0xf5d6)
    }

    /// Cupertino icon named "chevron_down_circle_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn chevron_down_circle_fill() -> IconData {
        cupertino(0xf5d7)
    }

    /// Cupertino icon named "chevron_down_square". Available on cupertino_icons package 1.0.0+ only.
    pub fn chevron_down_square() -> IconData {
        cupertino(0xf5d8)
    }

    /// Cupertino icon named "chevron_down_square_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn chevron_down_square_fill() -> IconData {
        cupertino(0xf5d9)
    }

    /// Cupertino icon named "chevron_forward". Available on cupertino_icons package 1.0.0+ only.
    /// This is the same icon as [`forward`](Self::forward()) which is available in cupertino_icons 0.1.3.
    pub fn chevron_forward() -> IconData {
        cupertino(0xf3d1)
    }

    /// Cupertino icon named "chevron_left". Available on cupertino_icons package 1.0.0+ only.
    /// This is the same icon as [`left_chevron`](Self::left_chevron()) which is available in cupertino_icons 0.1.3.
    pub fn chevron_left() -> IconData {
        cupertino(0xf3d2)
    }

    /// Cupertino icon named "chevron_left_2". Available on cupertino_icons package 1.0.0+ only.
    pub fn chevron_left_2() -> IconData {
        cupertino(0xf5da)
    }

    /// Cupertino icon named "chevron_left_circle". Available on cupertino_icons package 1.0.0+ only.
    pub fn chevron_left_circle() -> IconData {
        cupertino(0xf5db)
    }

    /// Cupertino icon named "chevron_left_circle_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn chevron_left_circle_fill() -> IconData {
        cupertino(0xf5dc)
    }

    /// Cupertino icon named "chevron_left_slash_chevron_right". Available on cupertino_icons package 1.0.0+ only.
    pub fn chevron_left_slash_chevron_right() -> IconData {
        cupertino(0xf5dd)
    }

    /// Cupertino icon named "chevron_left_square". Available on cupertino_icons package 1.0.0+ only.
    pub fn chevron_left_square() -> IconData {
        cupertino(0xf5de)
    }

    /// Cupertino icon named "chevron_left_square_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn chevron_left_square_fill() -> IconData {
        cupertino(0xf5df)
    }

    /// Cupertino icon named "chevron_right". Available on cupertino_icons package 1.0.0+ only.
    /// This is the same icon as [`right_chevron`](Self::right_chevron()) which is available in cupertino_icons 0.1.3.
    pub fn chevron_right() -> IconData {
        cupertino(0xf3d3)
    }

    /// Cupertino icon named "chevron_right_2". Available on cupertino_icons package 1.0.0+ only.
    pub fn chevron_right_2() -> IconData {
        cupertino(0xf5e0)
    }

    /// Cupertino icon named "chevron_right_circle". Available on cupertino_icons package 1.0.0+ only.
    pub fn chevron_right_circle() -> IconData {
        cupertino(0xf5e1)
    }

    /// Cupertino icon named "chevron_right_circle_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn chevron_right_circle_fill() -> IconData {
        cupertino(0xf5e2)
    }

    /// Cupertino icon named "chevron_right_square". Available on cupertino_icons package 1.0.0+ only.
    pub fn chevron_right_square() -> IconData {
        cupertino(0xf5e3)
    }

    /// Cupertino icon named "chevron_right_square_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn chevron_right_square_fill() -> IconData {
        cupertino(0xf5e4)
    }

    /// Cupertino icon named "chevron_up". Available on cupertino_icons package 1.0.0+ only.
    pub fn chevron_up() -> IconData {
        cupertino(0xf5e5)
    }

    /// Cupertino icon named "chevron_up_chevron_down". Available on cupertino_icons package 1.0.0+ only.
    pub fn chevron_up_chevron_down() -> IconData {
        cupertino(0xf5e6)
    }

    /// Cupertino icon named "chevron_up_circle". Available on cupertino_icons package 1.0.0+ only.
    pub fn chevron_up_circle() -> IconData {
        cupertino(0xf5e7)
    }

    /// Cupertino icon named "chevron_up_circle_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn chevron_up_circle_fill() -> IconData {
        cupertino(0xf5e8)
    }

    /// Cupertino icon named "chevron_up_square". Available on cupertino_icons package 1.0.0+ only.
    pub fn chevron_up_square() -> IconData {
        cupertino(0xf5e9)
    }

    /// Cupertino icon named "chevron_up_square_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn chevron_up_square_fill() -> IconData {
        cupertino(0xf5ea)
    }

    /// Cupertino icon named "circle_bottomthird_split". Available on cupertino_icons package 1.0.0+ only.
    pub fn circle_bottomthird_split() -> IconData {
        cupertino(0xf5eb)
    }

    /// Cupertino icon named "circle_fill". Available on cupertino_icons package 1.0.0+ only.
    /// This is the same icon as [`circle_filled`](Self::circle_filled()) which is available in cupertino_icons 0.1.3.
    pub fn circle_fill() -> IconData {
        cupertino(0xf400)
    }

    /// Cupertino icon named "circle_grid_3x3". Available on cupertino_icons package 1.0.0+ only.
    pub fn circle_grid_3x3() -> IconData {
        cupertino(0xf5ec)
    }

    /// Cupertino icon named "circle_grid_3x3_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn circle_grid_3x3_fill() -> IconData {
        cupertino(0xf5ed)
    }

    /// Cupertino icon named "circle_grid_hex". Available on cupertino_icons package 1.0.0+ only.
    pub fn circle_grid_hex() -> IconData {
        cupertino(0xf5ee)
    }

    /// Cupertino icon named "circle_grid_hex_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn circle_grid_hex_fill() -> IconData {
        cupertino(0xf5ef)
    }

    /// Cupertino icon named "circle_lefthalf_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn circle_lefthalf_fill() -> IconData {
        cupertino(0xf5f0)
    }

    /// Cupertino icon named "circle_righthalf_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn circle_righthalf_fill() -> IconData {
        cupertino(0xf5f1)
    }

    /// Cupertino icon named "clear_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn clear_fill() -> IconData {
        cupertino(0xf5f3)
    }

    /// Cupertino icon named "clock_fill". Available on cupertino_icons package 1.0.0+ only.
    /// This is the same icon as [`clock_solid`](Self::clock_solid()) which is available in cupertino_icons 0.1.3.
    /// This is the same icon as [`time_solid`](Self::time_solid()) which is available in cupertino_icons 0.1.3.
    pub fn clock_fill() -> IconData {
        cupertino(0xf403)
    }

    /// Cupertino icon named "cloud". Available on cupertino_icons package 1.0.0+ only.
    pub fn cloud() -> IconData {
        cupertino(0xf5f4)
    }

    /// Cupertino icon named "cloud_bolt". Available on cupertino_icons package 1.0.0+ only.
    pub fn cloud_bolt() -> IconData {
        cupertino(0xf5f5)
    }

    /// Cupertino icon named "cloud_bolt_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn cloud_bolt_fill() -> IconData {
        cupertino(0xf5f6)
    }

    /// Cupertino icon named "cloud_bolt_rain". Available on cupertino_icons package 1.0.0+ only.
    pub fn cloud_bolt_rain() -> IconData {
        cupertino(0xf5f7)
    }

    /// Cupertino icon named "cloud_bolt_rain_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn cloud_bolt_rain_fill() -> IconData {
        cupertino(0xf5f8)
    }

    /// Cupertino icon named "cloud_download". Available on cupertino_icons package 1.0.0+ only.
    pub fn cloud_download() -> IconData {
        cupertino(0xf8c4)
    }

    /// Cupertino icon named "cloud_download_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn cloud_download_fill() -> IconData {
        cupertino(0xf8c5)
    }

    /// Cupertino icon named "cloud_drizzle". Available on cupertino_icons package 1.0.0+ only.
    pub fn cloud_drizzle() -> IconData {
        cupertino(0xf5f9)
    }

    /// Cupertino icon named "cloud_drizzle_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn cloud_drizzle_fill() -> IconData {
        cupertino(0xf5fa)
    }

    /// Cupertino icon named "cloud_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn cloud_fill() -> IconData {
        cupertino(0xf5fb)
    }

    /// Cupertino icon named "cloud_fog". Available on cupertino_icons package 1.0.0+ only.
    pub fn cloud_fog() -> IconData {
        cupertino(0xf5fc)
    }

    /// Cupertino icon named "cloud_fog_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn cloud_fog_fill() -> IconData {
        cupertino(0xf5fd)
    }

    /// Cupertino icon named "cloud_hail". Available on cupertino_icons package 1.0.0+ only.
    pub fn cloud_hail() -> IconData {
        cupertino(0xf5fe)
    }

    /// Cupertino icon named "cloud_hail_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn cloud_hail_fill() -> IconData {
        cupertino(0xf5ff)
    }

    /// Cupertino icon named "cloud_heavyrain". Available on cupertino_icons package 1.0.0+ only.
    pub fn cloud_heavyrain() -> IconData {
        cupertino(0xf600)
    }

    /// Cupertino icon named "cloud_heavyrain_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn cloud_heavyrain_fill() -> IconData {
        cupertino(0xf601)
    }

    /// Cupertino icon named "cloud_moon". Available on cupertino_icons package 1.0.0+ only.
    pub fn cloud_moon() -> IconData {
        cupertino(0xf602)
    }

    /// Cupertino icon named "cloud_moon_bolt". Available on cupertino_icons package 1.0.0+ only.
    pub fn cloud_moon_bolt() -> IconData {
        cupertino(0xf603)
    }

    /// Cupertino icon named "cloud_moon_bolt_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn cloud_moon_bolt_fill() -> IconData {
        cupertino(0xf604)
    }

    /// Cupertino icon named "cloud_moon_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn cloud_moon_fill() -> IconData {
        cupertino(0xf605)
    }

    /// Cupertino icon named "cloud_moon_rain". Available on cupertino_icons package 1.0.0+ only.
    pub fn cloud_moon_rain() -> IconData {
        cupertino(0xf606)
    }

    /// Cupertino icon named "cloud_moon_rain_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn cloud_moon_rain_fill() -> IconData {
        cupertino(0xf607)
    }

    /// Cupertino icon named "cloud_rain". Available on cupertino_icons package 1.0.0+ only.
    pub fn cloud_rain() -> IconData {
        cupertino(0xf608)
    }

    /// Cupertino icon named "cloud_rain_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn cloud_rain_fill() -> IconData {
        cupertino(0xf609)
    }

    /// Cupertino icon named "cloud_sleet". Available on cupertino_icons package 1.0.0+ only.
    pub fn cloud_sleet() -> IconData {
        cupertino(0xf60a)
    }

    /// Cupertino icon named "cloud_sleet_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn cloud_sleet_fill() -> IconData {
        cupertino(0xf60b)
    }

    /// Cupertino icon named "cloud_snow". Available on cupertino_icons package 1.0.0+ only.
    pub fn cloud_snow() -> IconData {
        cupertino(0xf60c)
    }

    /// Cupertino icon named "cloud_snow_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn cloud_snow_fill() -> IconData {
        cupertino(0xf60d)
    }

    /// Cupertino icon named "cloud_sun". Available on cupertino_icons package 1.0.0+ only.
    pub fn cloud_sun() -> IconData {
        cupertino(0xf60e)
    }

    /// Cupertino icon named "cloud_sun_bolt". Available on cupertino_icons package 1.0.0+ only.
    pub fn cloud_sun_bolt() -> IconData {
        cupertino(0xf60f)
    }

    /// Cupertino icon named "cloud_sun_bolt_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn cloud_sun_bolt_fill() -> IconData {
        cupertino(0xf610)
    }

    /// Cupertino icon named "cloud_sun_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn cloud_sun_fill() -> IconData {
        cupertino(0xf611)
    }

    /// Cupertino icon named "cloud_sun_rain". Available on cupertino_icons package 1.0.0+ only.
    pub fn cloud_sun_rain() -> IconData {
        cupertino(0xf612)
    }

    /// Cupertino icon named "cloud_sun_rain_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn cloud_sun_rain_fill() -> IconData {
        cupertino(0xf613)
    }

    /// Cupertino icon named "cloud_upload". Available on cupertino_icons package 1.0.0+ only.
    pub fn cloud_upload() -> IconData {
        cupertino(0xf8c6)
    }

    /// Cupertino icon named "cloud_upload_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn cloud_upload_fill() -> IconData {
        cupertino(0xf8c7)
    }

    /// Cupertino icon named "color_filter". Available on cupertino_icons package 1.0.0+ only.
    pub fn color_filter() -> IconData {
        cupertino(0xf8c8)
    }

    /// Cupertino icon named "color_filter_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn color_filter_fill() -> IconData {
        cupertino(0xf8c9)
    }

    /// Cupertino icon named "command". Available on cupertino_icons package 1.0.0+ only.
    pub fn command() -> IconData {
        cupertino(0xf614)
    }

    /// Cupertino icon named "compass". Available on cupertino_icons package 1.0.0+ only.
    pub fn compass() -> IconData {
        cupertino(0xf8ca)
    }

    /// Cupertino icon named "compass_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn compass_fill() -> IconData {
        cupertino(0xf8cb)
    }

    /// Cupertino icon named "control". Available on cupertino_icons package 1.0.0+ only.
    pub fn control() -> IconData {
        cupertino(0xf615)
    }

    /// Cupertino icon named "creditcard". Available on cupertino_icons package 1.0.0+ only.
    pub fn creditcard() -> IconData {
        cupertino(0xf616)
    }

    /// Cupertino icon named "creditcard_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn creditcard_fill() -> IconData {
        cupertino(0xf617)
    }

    /// Cupertino icon named "crop". Available on cupertino_icons package 1.0.0+ only.
    pub fn crop() -> IconData {
        cupertino(0xf618)
    }

    /// Cupertino icon named "crop_rotate". Available on cupertino_icons package 1.0.0+ only.
    pub fn crop_rotate() -> IconData {
        cupertino(0xf619)
    }

    /// Cupertino icon named "cube". Available on cupertino_icons package 1.0.0+ only.
    pub fn cube() -> IconData {
        cupertino(0xf61a)
    }

    /// Cupertino icon named "cube_box". Available on cupertino_icons package 1.0.0+ only.
    pub fn cube_box() -> IconData {
        cupertino(0xf61b)
    }

    /// Cupertino icon named "cube_box_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn cube_box_fill() -> IconData {
        cupertino(0xf61c)
    }

    /// Cupertino icon named "cube_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn cube_fill() -> IconData {
        cupertino(0xf61d)
    }

    /// Cupertino icon named "cursor_rays". Available on cupertino_icons package 1.0.0+ only.
    pub fn cursor_rays() -> IconData {
        cupertino(0xf61e)
    }

    /// Cupertino icon named "decrease_indent". Available on cupertino_icons package 1.0.0+ only.
    pub fn decrease_indent() -> IconData {
        cupertino(0xf61f)
    }

    /// Cupertino icon named "decrease_quotelevel". Available on cupertino_icons package 1.0.0+ only.
    pub fn decrease_quotelevel() -> IconData {
        cupertino(0xf620)
    }

    /// Cupertino icon named "delete_left". Available on cupertino_icons package 1.0.0+ only.
    pub fn delete_left() -> IconData {
        cupertino(0xf621)
    }

    /// Cupertino icon named "delete_left_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn delete_left_fill() -> IconData {
        cupertino(0xf622)
    }

    /// Cupertino icon named "delete_right". Available on cupertino_icons package 1.0.0+ only.
    pub fn delete_right() -> IconData {
        cupertino(0xf623)
    }

    /// Cupertino icon named "delete_right_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn delete_right_fill() -> IconData {
        cupertino(0xf624)
    }

    /// Cupertino icon named "desktopcomputer". Available on cupertino_icons package 1.0.0+ only.
    pub fn desktopcomputer() -> IconData {
        cupertino(0xf625)
    }

    /// Cupertino icon named "device_desktop". Available on cupertino_icons package 1.0.0+ only.
    pub fn device_desktop() -> IconData {
        cupertino(0xf8cc)
    }

    /// Cupertino icon named "device_laptop". Available on cupertino_icons package 1.0.0+ only.
    pub fn device_laptop() -> IconData {
        cupertino(0xf8cd)
    }

    /// Cupertino icon named "device_phone_landscape". Available on cupertino_icons package 1.0.0+ only.
    pub fn device_phone_landscape() -> IconData {
        cupertino(0xf8ce)
    }

    /// Cupertino icon named "device_phone_portrait". Available on cupertino_icons package 1.0.0+ only.
    pub fn device_phone_portrait() -> IconData {
        cupertino(0xf8cf)
    }

    /// Cupertino icon named "dial". Available on cupertino_icons package 1.0.0+ only.
    pub fn dial() -> IconData {
        cupertino(0xf626)
    }

    /// Cupertino icon named "dial_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn dial_fill() -> IconData {
        cupertino(0xf627)
    }

    /// Cupertino icon named "divide". Available on cupertino_icons package 1.0.0+ only.
    pub fn divide() -> IconData {
        cupertino(0xf628)
    }

    /// Cupertino icon named "divide_circle". Available on cupertino_icons package 1.0.0+ only.
    pub fn divide_circle() -> IconData {
        cupertino(0xf629)
    }

    /// Cupertino icon named "divide_circle_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn divide_circle_fill() -> IconData {
        cupertino(0xf62a)
    }

    /// Cupertino icon named "divide_square". Available on cupertino_icons package 1.0.0+ only.
    pub fn divide_square() -> IconData {
        cupertino(0xf62b)
    }

    /// Cupertino icon named "divide_square_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn divide_square_fill() -> IconData {
        cupertino(0xf62c)
    }

    /// Cupertino icon named "doc". Available on cupertino_icons package 1.0.0+ only.
    pub fn doc() -> IconData {
        cupertino(0xf62d)
    }

    /// Cupertino icon named "doc_append". Available on cupertino_icons package 1.0.0+ only.
    pub fn doc_append() -> IconData {
        cupertino(0xf62e)
    }

    /// Cupertino icon named "doc_chart". Available on cupertino_icons package 1.0.0+ only.
    pub fn doc_chart() -> IconData {
        cupertino(0xf8d0)
    }

    /// Cupertino icon named "doc_chart_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn doc_chart_fill() -> IconData {
        cupertino(0xf8d1)
    }

    /// Cupertino icon named "doc_checkmark". Available on cupertino_icons package 1.0.0+ only.
    pub fn doc_checkmark() -> IconData {
        cupertino(0xf8d2)
    }

    /// Cupertino icon named "doc_checkmark_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn doc_checkmark_fill() -> IconData {
        cupertino(0xf8d3)
    }

    /// Cupertino icon named "doc_circle". Available on cupertino_icons package 1.0.0+ only.
    pub fn doc_circle() -> IconData {
        cupertino(0xf62f)
    }

    /// Cupertino icon named "doc_circle_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn doc_circle_fill() -> IconData {
        cupertino(0xf630)
    }

    /// Cupertino icon named "doc_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn doc_fill() -> IconData {
        cupertino(0xf631)
    }

    /// Cupertino icon named "doc_on_clipboard". Available on cupertino_icons package 1.0.0+ only.
    pub fn doc_on_clipboard() -> IconData {
        cupertino(0xf632)
    }

    /// Cupertino icon named "doc_on_clipboard_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn doc_on_clipboard_fill() -> IconData {
        cupertino(0xf633)
    }

    /// Cupertino icon named "doc_on_doc". Available on cupertino_icons package 1.0.0+ only.
    pub fn doc_on_doc() -> IconData {
        cupertino(0xf634)
    }

    /// Cupertino icon named "doc_on_doc_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn doc_on_doc_fill() -> IconData {
        cupertino(0xf635)
    }

    /// Cupertino icon named "doc_person". Available on cupertino_icons package 1.0.0+ only.
    pub fn doc_person() -> IconData {
        cupertino(0xf8d4)
    }

    /// Cupertino icon named "doc_person_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn doc_person_fill() -> IconData {
        cupertino(0xf8d5)
    }

    /// Cupertino icon named "doc_plaintext". Available on cupertino_icons package 1.0.0+ only.
    pub fn doc_plaintext() -> IconData {
        cupertino(0xf636)
    }

    /// Cupertino icon named "doc_richtext". Available on cupertino_icons package 1.0.0+ only.
    pub fn doc_richtext() -> IconData {
        cupertino(0xf637)
    }

    /// Cupertino icon named "doc_text". Available on cupertino_icons package 1.0.0+ only.
    pub fn doc_text() -> IconData {
        cupertino(0xf638)
    }

    /// Cupertino icon named "doc_text_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn doc_text_fill() -> IconData {
        cupertino(0xf639)
    }

    /// Cupertino icon named "doc_text_search". Available on cupertino_icons package 1.0.0+ only.
    pub fn doc_text_search() -> IconData {
        cupertino(0xf63a)
    }

    /// Cupertino icon named "doc_text_viewfinder". Available on cupertino_icons package 1.0.0+ only.
    pub fn doc_text_viewfinder() -> IconData {
        cupertino(0xf63b)
    }

    /// Cupertino icon named "dot_radiowaves_left_right". Available on cupertino_icons package 1.0.0+ only.
    pub fn dot_radiowaves_left_right() -> IconData {
        cupertino(0xf63c)
    }

    /// Cupertino icon named "dot_radiowaves_right". Available on cupertino_icons package 1.0.0+ only.
    pub fn dot_radiowaves_right() -> IconData {
        cupertino(0xf63d)
    }

    /// Cupertino icon named "dot_square". Available on cupertino_icons package 1.0.0+ only.
    pub fn dot_square() -> IconData {
        cupertino(0xf63e)
    }

    /// Cupertino icon named "dot_square_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn dot_square_fill() -> IconData {
        cupertino(0xf63f)
    }

    /// Cupertino icon named "download_circle". Available on cupertino_icons package 1.0.0+ only.
    pub fn download_circle() -> IconData {
        cupertino(0xf8d6)
    }

    /// Cupertino icon named "download_circle_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn download_circle_fill() -> IconData {
        cupertino(0xf8d7)
    }

    /// Cupertino icon named "drop". Available on cupertino_icons package 1.0.0+ only.
    pub fn drop() -> IconData {
        cupertino(0xf8d8)
    }

    /// Cupertino icon named "drop_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn drop_fill() -> IconData {
        cupertino(0xf8d9)
    }

    /// Cupertino icon named "drop_triangle". Available on cupertino_icons package 1.0.0+ only.
    pub fn drop_triangle() -> IconData {
        cupertino(0xf640)
    }

    /// Cupertino icon named "drop_triangle_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn drop_triangle_fill() -> IconData {
        cupertino(0xf641)
    }

    /// Cupertino icon named "ear". Available on cupertino_icons package 1.0.0+ only.
    pub fn ear() -> IconData {
        cupertino(0xf642)
    }

    /// Cupertino icon named "eject". Available on cupertino_icons package 1.0.0+ only.
    pub fn eject() -> IconData {
        cupertino(0xf643)
    }

    /// Cupertino icon named "eject_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn eject_fill() -> IconData {
        cupertino(0xf644)
    }

    /// Cupertino icon named "ellipses_bubble". Available on cupertino_icons package 1.0.0+ only.
    pub fn ellipses_bubble() -> IconData {
        cupertino(0xf645)
    }

    /// Cupertino icon named "ellipses_bubble_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn ellipses_bubble_fill() -> IconData {
        cupertino(0xf646)
    }

    /// Cupertino icon named "ellipsis_circle". Available on cupertino_icons package 1.0.0+ only.
    pub fn ellipsis_circle() -> IconData {
        cupertino(0xf647)
    }

    /// Cupertino icon named "ellipsis_circle_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn ellipsis_circle_fill() -> IconData {
        cupertino(0xf648)
    }

    /// Cupertino icon named "ellipsis_vertical". Available on cupertino_icons package 1.0.0+ only.
    pub fn ellipsis_vertical() -> IconData {
        cupertino(0xf8da)
    }

    /// Cupertino icon named "ellipsis_vertical_circle". Available on cupertino_icons package 1.0.0+ only.
    pub fn ellipsis_vertical_circle() -> IconData {
        cupertino(0xf8db)
    }

    /// Cupertino icon named "ellipsis_vertical_circle_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn ellipsis_vertical_circle_fill() -> IconData {
        cupertino(0xf8dc)
    }

    /// Cupertino icon named "envelope". Available on cupertino_icons package 1.0.0+ only.
    /// This is the same icon as [`mail`](Self::mail()) which is available in cupertino_icons 0.1.3.
    pub fn envelope() -> IconData {
        cupertino(0xf422)
    }

    /// Cupertino icon named "envelope_badge". Available on cupertino_icons package 1.0.0+ only.
    pub fn envelope_badge() -> IconData {
        cupertino(0xf649)
    }

    /// Cupertino icon named "envelope_badge_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn envelope_badge_fill() -> IconData {
        cupertino(0xf64a)
    }

    /// Cupertino icon named "envelope_circle". Available on cupertino_icons package 1.0.0+ only.
    pub fn envelope_circle() -> IconData {
        cupertino(0xf64b)
    }

    /// Cupertino icon named "envelope_circle_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn envelope_circle_fill() -> IconData {
        cupertino(0xf64c)
    }

    /// Cupertino icon named "envelope_fill". Available on cupertino_icons package 1.0.0+ only.
    /// This is the same icon as [`mail_solid`](Self::mail_solid()) which is available in cupertino_icons 0.1.3.
    pub fn envelope_fill() -> IconData {
        cupertino(0xf423)
    }

    /// Cupertino icon named "envelope_open". Available on cupertino_icons package 1.0.0+ only.
    pub fn envelope_open() -> IconData {
        cupertino(0xf64d)
    }

    /// Cupertino icon named "envelope_open_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn envelope_open_fill() -> IconData {
        cupertino(0xf64e)
    }

    /// Cupertino icon named "equal". Available on cupertino_icons package 1.0.0+ only.
    pub fn equal() -> IconData {
        cupertino(0xf64f)
    }

    /// Cupertino icon named "equal_circle". Available on cupertino_icons package 1.0.0+ only.
    pub fn equal_circle() -> IconData {
        cupertino(0xf650)
    }

    /// Cupertino icon named "equal_circle_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn equal_circle_fill() -> IconData {
        cupertino(0xf651)
    }

    /// Cupertino icon named "equal_square". Available on cupertino_icons package 1.0.0+ only.
    pub fn equal_square() -> IconData {
        cupertino(0xf652)
    }

    /// Cupertino icon named "equal_square_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn equal_square_fill() -> IconData {
        cupertino(0xf653)
    }

    /// Cupertino icon named "escape". Available on cupertino_icons package 1.0.0+ only.
    pub fn escape() -> IconData {
        cupertino(0xf654)
    }

    /// Cupertino icon named "exclamationmark". Available on cupertino_icons package 1.0.0+ only.
    pub fn exclamationmark() -> IconData {
        cupertino(0xf655)
    }

    /// Cupertino icon named "exclamationmark_bubble". Available on cupertino_icons package 1.0.0+ only.
    pub fn exclamationmark_bubble() -> IconData {
        cupertino(0xf656)
    }

    /// Cupertino icon named "exclamationmark_bubble_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn exclamationmark_bubble_fill() -> IconData {
        cupertino(0xf657)
    }

    /// Cupertino icon named "exclamationmark_circle". Available on cupertino_icons package 1.0.0+ only.
    pub fn exclamationmark_circle() -> IconData {
        cupertino(0xf658)
    }

    /// Cupertino icon named "exclamationmark_circle_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn exclamationmark_circle_fill() -> IconData {
        cupertino(0xf659)
    }

    /// Cupertino icon named "exclamationmark_octagon". Available on cupertino_icons package 1.0.0+ only.
    pub fn exclamationmark_octagon() -> IconData {
        cupertino(0xf65a)
    }

    /// Cupertino icon named "exclamationmark_octagon_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn exclamationmark_octagon_fill() -> IconData {
        cupertino(0xf65b)
    }

    /// Cupertino icon named "exclamationmark_shield". Available on cupertino_icons package 1.0.0+ only.
    pub fn exclamationmark_shield() -> IconData {
        cupertino(0xf65c)
    }

    /// Cupertino icon named "exclamationmark_shield_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn exclamationmark_shield_fill() -> IconData {
        cupertino(0xf65d)
    }

    /// Cupertino icon named "exclamationmark_square". Available on cupertino_icons package 1.0.0+ only.
    pub fn exclamationmark_square() -> IconData {
        cupertino(0xf65e)
    }

    /// Cupertino icon named "exclamationmark_square_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn exclamationmark_square_fill() -> IconData {
        cupertino(0xf65f)
    }

    /// Cupertino icon named "exclamationmark_triangle". Available on cupertino_icons package 1.0.0+ only.
    pub fn exclamationmark_triangle() -> IconData {
        cupertino(0xf660)
    }

    /// Cupertino icon named "exclamationmark_triangle_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn exclamationmark_triangle_fill() -> IconData {
        cupertino(0xf661)
    }

    /// Cupertino icon named "eye_fill". Available on cupertino_icons package 1.0.0+ only.
    /// This is the same icon as [`eye_solid`](Self::eye_solid()) which is available in cupertino_icons 0.1.3.
    pub fn eye_fill() -> IconData {
        cupertino(0xf425)
    }

    /// Cupertino icon named "eye_slash". Available on cupertino_icons package 1.0.0+ only.
    pub fn eye_slash() -> IconData {
        cupertino(0xf662)
    }

    /// Cupertino icon named "eye_slash_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn eye_slash_fill() -> IconData {
        cupertino(0xf663)
    }

    /// Cupertino icon named "eyedropper". Available on cupertino_icons package 1.0.0+ only.
    pub fn eyedropper() -> IconData {
        cupertino(0xf664)
    }

    /// Cupertino icon named "eyedropper_full". Available on cupertino_icons package 1.0.0+ only.
    pub fn eyedropper_full() -> IconData {
        cupertino(0xf665)
    }

    /// Cupertino icon named "eyedropper_halffull". Available on cupertino_icons package 1.0.0+ only.
    pub fn eyedropper_halffull() -> IconData {
        cupertino(0xf666)
    }

    /// Cupertino icon named "eyeglasses". Available on cupertino_icons package 1.0.0+ only.
    pub fn eyeglasses() -> IconData {
        cupertino(0xf667)
    }

    /// Cupertino icon named "f_cursive". Available on cupertino_icons package 1.0.0+ only.
    pub fn f_cursive() -> IconData {
        cupertino(0xf668)
    }

    /// Cupertino icon named "f_cursive_circle". Available on cupertino_icons package 1.0.0+ only.
    pub fn f_cursive_circle() -> IconData {
        cupertino(0xf669)
    }

    /// Cupertino icon named "f_cursive_circle_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn f_cursive_circle_fill() -> IconData {
        cupertino(0xf66a)
    }

    /// Cupertino icon named "film". Available on cupertino_icons package 1.0.0+ only.
    pub fn film() -> IconData {
        cupertino(0xf66b)
    }

    /// Cupertino icon named "film_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn film_fill() -> IconData {
        cupertino(0xf66c)
    }

    /// Cupertino icon named "flag_circle". Available on cupertino_icons package 1.0.0+ only.
    pub fn flag_circle() -> IconData {
        cupertino(0xf66d)
    }

    /// Cupertino icon named "flag_circle_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn flag_circle_fill() -> IconData {
        cupertino(0xf66e)
    }

    /// Cupertino icon named "flag_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn flag_fill() -> IconData {
        cupertino(0xf66f)
    }

    /// Cupertino icon named "flag_slash". Available on cupertino_icons package 1.0.0+ only.
    pub fn flag_slash() -> IconData {
        cupertino(0xf670)
    }

    /// Cupertino icon named "flag_slash_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn flag_slash_fill() -> IconData {
        cupertino(0xf671)
    }

    /// Cupertino icon named "flame". Available on cupertino_icons package 1.0.0+ only.
    pub fn flame() -> IconData {
        cupertino(0xf672)
    }

    /// Cupertino icon named "flame_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn flame_fill() -> IconData {
        cupertino(0xf673)
    }

    /// Cupertino icon named "floppy_disk". Available on cupertino_icons package 1.0.0+ only.
    pub fn floppy_disk() -> IconData {
        cupertino(0xf8dd)
    }

    /// Cupertino icon named "flowchart". Available on cupertino_icons package 1.0.0+ only.
    pub fn flowchart() -> IconData {
        cupertino(0xf674)
    }

    /// Cupertino icon named "flowchart_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn flowchart_fill() -> IconData {
        cupertino(0xf675)
    }

    /// Cupertino icon named "folder_badge_minus". Available on cupertino_icons package 1.0.0+ only.
    pub fn folder_badge_minus() -> IconData {
        cupertino(0xf676)
    }

    /// Cupertino icon named "folder_badge_person_crop". Available on cupertino_icons package 1.0.0+ only.
    pub fn folder_badge_person_crop() -> IconData {
        cupertino(0xf677)
    }

    /// Cupertino icon named "folder_badge_plus". Available on cupertino_icons package 1.0.0+ only.
    pub fn folder_badge_plus() -> IconData {
        cupertino(0xf678)
    }

    /// Cupertino icon named "folder_circle". Available on cupertino_icons package 1.0.0+ only.
    pub fn folder_circle() -> IconData {
        cupertino(0xf679)
    }

    /// Cupertino icon named "folder_circle_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn folder_circle_fill() -> IconData {
        cupertino(0xf67a)
    }

    /// Cupertino icon named "folder_fill". Available on cupertino_icons package 1.0.0+ only.
    /// This is the same icon as [`folder_solid`](Self::folder_solid()) which is available in cupertino_icons 0.1.3.
    pub fn folder_fill() -> IconData {
        cupertino(0xf435)
    }

    /// Cupertino icon named "folder_fill_badge_minus". Available on cupertino_icons package 1.0.0+ only.
    pub fn folder_fill_badge_minus() -> IconData {
        cupertino(0xf67b)
    }

    /// Cupertino icon named "folder_fill_badge_person_crop". Available on cupertino_icons package 1.0.0+ only.
    pub fn folder_fill_badge_person_crop() -> IconData {
        cupertino(0xf67c)
    }

    /// Cupertino icon named "folder_fill_badge_plus". Available on cupertino_icons package 1.0.0+ only.
    pub fn folder_fill_badge_plus() -> IconData {
        cupertino(0xf67d)
    }

    /// Cupertino icon named "forward_end". Available on cupertino_icons package 1.0.0+ only.
    pub fn forward_end() -> IconData {
        cupertino(0xf67f)
    }

    /// Cupertino icon named "forward_end_alt". Available on cupertino_icons package 1.0.0+ only.
    pub fn forward_end_alt() -> IconData {
        cupertino(0xf680)
    }

    /// Cupertino icon named "forward_end_alt_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn forward_end_alt_fill() -> IconData {
        cupertino(0xf681)
    }

    /// Cupertino icon named "forward_end_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn forward_end_fill() -> IconData {
        cupertino(0xf682)
    }

    /// Cupertino icon named "forward_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn forward_fill() -> IconData {
        cupertino(0xf683)
    }

    /// Cupertino icon named "function". Available on cupertino_icons package 1.0.0+ only.
    pub fn function() -> IconData {
        cupertino(0xf684)
    }

    /// Cupertino icon named "fx". Available on cupertino_icons package 1.0.0+ only.
    pub fn fx() -> IconData {
        cupertino(0xf685)
    }

    /// Cupertino icon named "gamecontroller". Available on cupertino_icons package 1.0.0+ only.
    /// This is the same icon as [`game_controller`](Self::game_controller()) which is available in cupertino_icons 0.1.3.
    pub fn gamecontroller() -> IconData {
        cupertino(0xf43a)
    }

    /// Cupertino icon named "gamecontroller_alt_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn gamecontroller_alt_fill() -> IconData {
        cupertino(0xf8de)
    }

    /// Cupertino icon named "gamecontroller_fill". Available on cupertino_icons package 1.0.0+ only.
    /// This is the same icon as [`game_controller_solid`](Self::game_controller_solid()) which is available in cupertino_icons 0.1.3.
    pub fn gamecontroller_fill() -> IconData {
        cupertino(0xf43b)
    }

    /// Cupertino icon named "gauge". Available on cupertino_icons package 1.0.0+ only.
    pub fn gauge() -> IconData {
        cupertino(0xf686)
    }

    /// Cupertino icon named "gauge_badge_minus". Available on cupertino_icons package 1.0.0+ only.
    pub fn gauge_badge_minus() -> IconData {
        cupertino(0xf687)
    }

    /// Cupertino icon named "gauge_badge_plus". Available on cupertino_icons package 1.0.0+ only.
    pub fn gauge_badge_plus() -> IconData {
        cupertino(0xf688)
    }

    /// Cupertino icon named "gear_alt". Available on cupertino_icons package 1.0.0+ only.
    /// This is the same icon as [`gear`](Self::gear()) which is available in cupertino_icons 0.1.3.
    /// This is the same icon as [`gear_big`](Self::gear_big()) which is available in cupertino_icons 0.1.3.
    pub fn gear_alt() -> IconData {
        cupertino(0xf43c)
    }

    /// Cupertino icon named "gear_alt_fill". Available on cupertino_icons package 1.0.0+ only.
    /// This is the same icon as [`gear_solid`](Self::gear_solid()) which is available in cupertino_icons 0.1.3.
    pub fn gear_alt_fill() -> IconData {
        cupertino(0xf43d)
    }

    /// Cupertino icon named "gift". Available on cupertino_icons package 1.0.0+ only.
    pub fn gift() -> IconData {
        cupertino(0xf689)
    }

    /// Cupertino icon named "gift_alt". Available on cupertino_icons package 1.0.0+ only.
    pub fn gift_alt() -> IconData {
        cupertino(0xf68a)
    }

    /// Cupertino icon named "gift_alt_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn gift_alt_fill() -> IconData {
        cupertino(0xf68b)
    }

    /// Cupertino icon named "gift_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn gift_fill() -> IconData {
        cupertino(0xf68c)
    }

    /// Cupertino icon named "globe". Available on cupertino_icons package 1.0.0+ only.
    pub fn globe() -> IconData {
        cupertino(0xf68d)
    }

    /// Cupertino icon named "gobackward". Available on cupertino_icons package 1.0.0+ only.
    pub fn gobackward() -> IconData {
        cupertino(0xf68e)
    }

    /// Cupertino icon named "gobackward_10". Available on cupertino_icons package 1.0.0+ only.
    pub fn gobackward_10() -> IconData {
        cupertino(0xf68f)
    }

    /// Cupertino icon named "gobackward_15". Available on cupertino_icons package 1.0.0+ only.
    pub fn gobackward_15() -> IconData {
        cupertino(0xf690)
    }

    /// Cupertino icon named "gobackward_30". Available on cupertino_icons package 1.0.0+ only.
    pub fn gobackward_30() -> IconData {
        cupertino(0xf691)
    }

    /// Cupertino icon named "gobackward_45". Available on cupertino_icons package 1.0.0+ only.
    pub fn gobackward_45() -> IconData {
        cupertino(0xf692)
    }

    /// Cupertino icon named "gobackward_60". Available on cupertino_icons package 1.0.0+ only.
    pub fn gobackward_60() -> IconData {
        cupertino(0xf693)
    }

    /// Cupertino icon named "gobackward_75". Available on cupertino_icons package 1.0.0+ only.
    pub fn gobackward_75() -> IconData {
        cupertino(0xf694)
    }

    /// Cupertino icon named "gobackward_90". Available on cupertino_icons package 1.0.0+ only.
    pub fn gobackward_90() -> IconData {
        cupertino(0xf695)
    }

    /// Cupertino icon named "gobackward_minus". Available on cupertino_icons package 1.0.0+ only.
    pub fn gobackward_minus() -> IconData {
        cupertino(0xf696)
    }

    /// Cupertino icon named "goforward". Available on cupertino_icons package 1.0.0+ only.
    pub fn goforward() -> IconData {
        cupertino(0xf697)
    }

    /// Cupertino icon named "goforward_10". Available on cupertino_icons package 1.0.0+ only.
    pub fn goforward_10() -> IconData {
        cupertino(0xf698)
    }

    /// Cupertino icon named "goforward_15". Available on cupertino_icons package 1.0.0+ only.
    pub fn goforward_15() -> IconData {
        cupertino(0xf699)
    }

    /// Cupertino icon named "goforward_30". Available on cupertino_icons package 1.0.0+ only.
    pub fn goforward_30() -> IconData {
        cupertino(0xf69a)
    }

    /// Cupertino icon named "goforward_45". Available on cupertino_icons package 1.0.0+ only.
    pub fn goforward_45() -> IconData {
        cupertino(0xf69b)
    }

    /// Cupertino icon named "goforward_60". Available on cupertino_icons package 1.0.0+ only.
    pub fn goforward_60() -> IconData {
        cupertino(0xf69c)
    }

    /// Cupertino icon named "goforward_75". Available on cupertino_icons package 1.0.0+ only.
    pub fn goforward_75() -> IconData {
        cupertino(0xf69d)
    }

    /// Cupertino icon named "goforward_90". Available on cupertino_icons package 1.0.0+ only.
    pub fn goforward_90() -> IconData {
        cupertino(0xf69e)
    }

    /// Cupertino icon named "goforward_plus". Available on cupertino_icons package 1.0.0+ only.
    pub fn goforward_plus() -> IconData {
        cupertino(0xf69f)
    }

    /// Cupertino icon named "graph_circle". Available on cupertino_icons package 1.0.0+ only.
    pub fn graph_circle() -> IconData {
        cupertino(0xf8df)
    }

    /// Cupertino icon named "graph_circle_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn graph_circle_fill() -> IconData {
        cupertino(0xf8e0)
    }

    /// Cupertino icon named "graph_square". Available on cupertino_icons package 1.0.0+ only.
    pub fn graph_square() -> IconData {
        cupertino(0xf8e1)
    }

    /// Cupertino icon named "graph_square_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn graph_square_fill() -> IconData {
        cupertino(0xf8e2)
    }

    /// Cupertino icon named "greaterthan". Available on cupertino_icons package 1.0.0+ only.
    pub fn greaterthan() -> IconData {
        cupertino(0xf6a0)
    }

    /// Cupertino icon named "greaterthan_circle". Available on cupertino_icons package 1.0.0+ only.
    pub fn greaterthan_circle() -> IconData {
        cupertino(0xf6a1)
    }

    /// Cupertino icon named "greaterthan_circle_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn greaterthan_circle_fill() -> IconData {
        cupertino(0xf6a2)
    }

    /// Cupertino icon named "greaterthan_square". Available on cupertino_icons package 1.0.0+ only.
    pub fn greaterthan_square() -> IconData {
        cupertino(0xf6a3)
    }

    /// Cupertino icon named "greaterthan_square_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn greaterthan_square_fill() -> IconData {
        cupertino(0xf6a4)
    }

    /// Cupertino icon named "grid". Available on cupertino_icons package 1.0.0+ only.
    pub fn grid() -> IconData {
        cupertino(0xf6a5)
    }

    /// Cupertino icon named "grid_circle". Available on cupertino_icons package 1.0.0+ only.
    pub fn grid_circle() -> IconData {
        cupertino(0xf6a6)
    }

    /// Cupertino icon named "grid_circle_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn grid_circle_fill() -> IconData {
        cupertino(0xf6a7)
    }

    /// Cupertino icon named "guitars". Available on cupertino_icons package 1.0.0+ only.
    pub fn guitars() -> IconData {
        cupertino(0xf6a8)
    }

    /// Cupertino icon named "hammer". Available on cupertino_icons package 1.0.0+ only.
    pub fn hammer() -> IconData {
        cupertino(0xf6a9)
    }

    /// Cupertino icon named "hammer_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn hammer_fill() -> IconData {
        cupertino(0xf6aa)
    }

    /// Cupertino icon named "hand_draw". Available on cupertino_icons package 1.0.0+ only.
    pub fn hand_draw() -> IconData {
        cupertino(0xf6ab)
    }

    /// Cupertino icon named "hand_draw_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn hand_draw_fill() -> IconData {
        cupertino(0xf6ac)
    }

    /// Cupertino icon named "hand_point_left". Available on cupertino_icons package 1.0.0+ only.
    pub fn hand_point_left() -> IconData {
        cupertino(0xf6ad)
    }

    /// Cupertino icon named "hand_point_left_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn hand_point_left_fill() -> IconData {
        cupertino(0xf6ae)
    }

    /// Cupertino icon named "hand_point_right". Available on cupertino_icons package 1.0.0+ only.
    pub fn hand_point_right() -> IconData {
        cupertino(0xf6af)
    }

    /// Cupertino icon named "hand_point_right_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn hand_point_right_fill() -> IconData {
        cupertino(0xf6b0)
    }

    /// Cupertino icon named "hand_raised". Available on cupertino_icons package 1.0.0+ only.
    pub fn hand_raised() -> IconData {
        cupertino(0xf6b1)
    }

    /// Cupertino icon named "hand_raised_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn hand_raised_fill() -> IconData {
        cupertino(0xf6b2)
    }

    /// Cupertino icon named "hand_raised_slash". Available on cupertino_icons package 1.0.0+ only.
    pub fn hand_raised_slash() -> IconData {
        cupertino(0xf6b3)
    }

    /// Cupertino icon named "hand_raised_slash_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn hand_raised_slash_fill() -> IconData {
        cupertino(0xf6b4)
    }

    /// Cupertino icon named "hand_thumbsdown". Available on cupertino_icons package 1.0.0+ only.
    pub fn hand_thumbsdown() -> IconData {
        cupertino(0xf6b5)
    }

    /// Cupertino icon named "hand_thumbsdown_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn hand_thumbsdown_fill() -> IconData {
        cupertino(0xf6b6)
    }

    /// Cupertino icon named "hand_thumbsup". Available on cupertino_icons package 1.0.0+ only.
    pub fn hand_thumbsup() -> IconData {
        cupertino(0xf6b7)
    }

    /// Cupertino icon named "hand_thumbsup_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn hand_thumbsup_fill() -> IconData {
        cupertino(0xf6b8)
    }

    /// Cupertino icon named "hare". Available on cupertino_icons package 1.0.0+ only.
    pub fn hare() -> IconData {
        cupertino(0xf6b9)
    }

    /// Cupertino icon named "hare_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn hare_fill() -> IconData {
        cupertino(0xf6ba)
    }

    /// Cupertino icon named "headphones". Available on cupertino_icons package 1.0.0+ only.
    pub fn headphones() -> IconData {
        cupertino(0xf6bb)
    }

    /// Cupertino icon named "heart_circle". Available on cupertino_icons package 1.0.0+ only.
    pub fn heart_circle() -> IconData {
        cupertino(0xf6bc)
    }

    /// Cupertino icon named "heart_circle_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn heart_circle_fill() -> IconData {
        cupertino(0xf6bd)
    }

    /// Cupertino icon named "heart_fill". Available on cupertino_icons package 1.0.0+ only.
    /// This is the same icon as [`heart_solid`](Self::heart_solid()) which is available in cupertino_icons 0.1.3.
    pub fn heart_fill() -> IconData {
        cupertino(0xf443)
    }

    /// Cupertino icon named "heart_slash". Available on cupertino_icons package 1.0.0+ only.
    pub fn heart_slash() -> IconData {
        cupertino(0xf6be)
    }

    /// Cupertino icon named "heart_slash_circle". Available on cupertino_icons package 1.0.0+ only.
    pub fn heart_slash_circle() -> IconData {
        cupertino(0xf6bf)
    }

    /// Cupertino icon named "heart_slash_circle_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn heart_slash_circle_fill() -> IconData {
        cupertino(0xf6c0)
    }

    /// Cupertino icon named "heart_slash_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn heart_slash_fill() -> IconData {
        cupertino(0xf6c1)
    }

    /// Cupertino icon named "helm". Available on cupertino_icons package 1.0.0+ only.
    pub fn helm() -> IconData {
        cupertino(0xf6c2)
    }

    /// Cupertino icon named "hexagon". Available on cupertino_icons package 1.0.0+ only.
    pub fn hexagon() -> IconData {
        cupertino(0xf6c3)
    }

    /// Cupertino icon named "hexagon_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn hexagon_fill() -> IconData {
        cupertino(0xf6c4)
    }

    /// Cupertino icon named "hifispeaker". Available on cupertino_icons package 1.0.0+ only.
    pub fn hifispeaker() -> IconData {
        cupertino(0xf6c5)
    }

    /// Cupertino icon named "hifispeaker_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn hifispeaker_fill() -> IconData {
        cupertino(0xf6c6)
    }

    /// Cupertino icon named "hourglass". Available on cupertino_icons package 1.0.0+ only.
    pub fn hourglass() -> IconData {
        cupertino(0xf6c7)
    }

    /// Cupertino icon named "hourglass_bottomhalf_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn hourglass_bottomhalf_fill() -> IconData {
        cupertino(0xf6c8)
    }

    /// Cupertino icon named "hourglass_tophalf_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn hourglass_tophalf_fill() -> IconData {
        cupertino(0xf6c9)
    }

    /// Cupertino icon named "house". Available on cupertino_icons package 1.0.0+ only.
    /// This is the same icon as [`home`](Self::home()) which is available in cupertino_icons 0.1.3.
    pub fn house() -> IconData {
        cupertino(0xf447)
    }

    /// Cupertino icon named "house_alt". Available on cupertino_icons package 1.0.0+ only.
    pub fn house_alt() -> IconData {
        cupertino(0xf8e3)
    }

    /// Cupertino icon named "house_alt_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn house_alt_fill() -> IconData {
        cupertino(0xf8e4)
    }

    /// Cupertino icon named "house_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn house_fill() -> IconData {
        cupertino(0xf6ca)
    }

    /// Cupertino icon named "hurricane". Available on cupertino_icons package 1.0.0+ only.
    pub fn hurricane() -> IconData {
        cupertino(0xf6cb)
    }

    /// Cupertino icon named "increase_indent". Available on cupertino_icons package 1.0.0+ only.
    pub fn increase_indent() -> IconData {
        cupertino(0xf6cc)
    }

    /// Cupertino icon named "increase_quotelevel". Available on cupertino_icons package 1.0.0+ only.
    pub fn increase_quotelevel() -> IconData {
        cupertino(0xf6cd)
    }

    /// Cupertino icon named "infinite". Available on cupertino_icons package 1.0.0+ only.
    /// This is the same icon as [`loop`](Self::loop()) which is available in cupertino_icons 0.1.3.
    /// This is the same icon as [`loop_thick`](Self::loop_thick()) which is available in cupertino_icons 0.1.3.
    pub fn infinite() -> IconData {
        cupertino(0xf449)
    }

    /// Cupertino icon named "info_circle". Available on cupertino_icons package 1.0.0+ only.
    /// This is the same icon as [`info`](Self::info()) which is available in cupertino_icons 0.1.3.
    pub fn info_circle() -> IconData {
        cupertino(0xf44c)
    }

    /// Cupertino icon named "info_circle_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn info_circle_fill() -> IconData {
        cupertino(0xf6cf)
    }

    /// Cupertino icon named "italic". Available on cupertino_icons package 1.0.0+ only.
    pub fn italic() -> IconData {
        cupertino(0xf6d0)
    }

    /// Cupertino icon named "keyboard". Available on cupertino_icons package 1.0.0+ only.
    pub fn keyboard() -> IconData {
        cupertino(0xf6d1)
    }

    /// Cupertino icon named "keyboard_chevron_compact_down". Available on cupertino_icons package 1.0.0+ only.
    pub fn keyboard_chevron_compact_down() -> IconData {
        cupertino(0xf6d2)
    }

    /// Cupertino icon named "largecircle_fill_circle". Available on cupertino_icons package 1.0.0+ only.
    pub fn largecircle_fill_circle() -> IconData {
        cupertino(0xf6d3)
    }

    /// Cupertino icon named "lasso". Available on cupertino_icons package 1.0.0+ only.
    pub fn lasso() -> IconData {
        cupertino(0xf6d4)
    }

    /// Cupertino icon named "layers". Available on cupertino_icons package 1.0.0+ only.
    pub fn layers() -> IconData {
        cupertino(0xf8e5)
    }

    /// Cupertino icon named "layers_alt". Available on cupertino_icons package 1.0.0+ only.
    pub fn layers_alt() -> IconData {
        cupertino(0xf8e6)
    }

    /// Cupertino icon named "layers_alt_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn layers_alt_fill() -> IconData {
        cupertino(0xf8e7)
    }

    /// Cupertino icon named "layers_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn layers_fill() -> IconData {
        cupertino(0xf8e8)
    }

    /// Cupertino icon named "leaf_arrow_circlepath". Available on cupertino_icons package 1.0.0+ only.
    pub fn leaf_arrow_circlepath() -> IconData {
        cupertino(0xf6d5)
    }

    /// Cupertino icon named "lessthan". Available on cupertino_icons package 1.0.0+ only.
    pub fn lessthan() -> IconData {
        cupertino(0xf6d6)
    }

    /// Cupertino icon named "lessthan_circle". Available on cupertino_icons package 1.0.0+ only.
    pub fn lessthan_circle() -> IconData {
        cupertino(0xf6d7)
    }

    /// Cupertino icon named "lessthan_circle_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn lessthan_circle_fill() -> IconData {
        cupertino(0xf6d8)
    }

    /// Cupertino icon named "lessthan_square". Available on cupertino_icons package 1.0.0+ only.
    pub fn lessthan_square() -> IconData {
        cupertino(0xf6d9)
    }

    /// Cupertino icon named "lessthan_square_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn lessthan_square_fill() -> IconData {
        cupertino(0xf6da)
    }

    /// Cupertino icon named "light_max". Available on cupertino_icons package 1.0.0+ only.
    pub fn light_max() -> IconData {
        cupertino(0xf6db)
    }

    /// Cupertino icon named "light_min". Available on cupertino_icons package 1.0.0+ only.
    pub fn light_min() -> IconData {
        cupertino(0xf6dc)
    }

    /// Cupertino icon named "lightbulb". Available on cupertino_icons package 1.0.0+ only.
    pub fn lightbulb() -> IconData {
        cupertino(0xf6dd)
    }

    /// Cupertino icon named "lightbulb_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn lightbulb_fill() -> IconData {
        cupertino(0xf6de)
    }

    /// Cupertino icon named "lightbulb_slash". Available on cupertino_icons package 1.0.0+ only.
    pub fn lightbulb_slash() -> IconData {
        cupertino(0xf6df)
    }

    /// Cupertino icon named "lightbulb_slash_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn lightbulb_slash_fill() -> IconData {
        cupertino(0xf6e0)
    }

    /// Cupertino icon named "line_horizontal_3". Available on cupertino_icons package 1.0.0+ only.
    pub fn line_horizontal_3() -> IconData {
        cupertino(0xf6e1)
    }

    /// Cupertino icon named "line_horizontal_3_decrease". Available on cupertino_icons package 1.0.0+ only.
    pub fn line_horizontal_3_decrease() -> IconData {
        cupertino(0xf6e2)
    }

    /// Cupertino icon named "line_horizontal_3_decrease_circle". Available on cupertino_icons package 1.0.0+ only.
    pub fn line_horizontal_3_decrease_circle() -> IconData {
        cupertino(0xf6e3)
    }

    /// Cupertino icon named "line_horizontal_3_decrease_circle_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn line_horizontal_3_decrease_circle_fill() -> IconData {
        cupertino(0xf6e4)
    }

    /// Cupertino icon named "link". Available on cupertino_icons package 1.0.0+ only.
    pub fn link() -> IconData {
        cupertino(0xf6e5)
    }

    /// Cupertino icon named "link_circle". Available on cupertino_icons package 1.0.0+ only.
    pub fn link_circle() -> IconData {
        cupertino(0xf6e6)
    }

    /// Cupertino icon named "link_circle_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn link_circle_fill() -> IconData {
        cupertino(0xf6e7)
    }

    /// Cupertino icon named "list_bullet". Available on cupertino_icons package 1.0.0+ only.
    pub fn list_bullet() -> IconData {
        cupertino(0xf6e8)
    }

    /// Cupertino icon named "list_bullet_below_rectangle". Available on cupertino_icons package 1.0.0+ only.
    pub fn list_bullet_below_rectangle() -> IconData {
        cupertino(0xf6e9)
    }

    /// Cupertino icon named "list_bullet_indent". Available on cupertino_icons package 1.0.0+ only.
    pub fn list_bullet_indent() -> IconData {
        cupertino(0xf6ea)
    }

    /// Cupertino icon named "list_dash". Available on cupertino_icons package 1.0.0+ only.
    pub fn list_dash() -> IconData {
        cupertino(0xf6eb)
    }

    /// Cupertino icon named "list_number". Available on cupertino_icons package 1.0.0+ only.
    pub fn list_number() -> IconData {
        cupertino(0xf6ec)
    }

    /// Cupertino icon named "list_number_rtl". Available on cupertino_icons package 1.0.0+ only.
    pub fn list_number_rtl() -> IconData {
        cupertino(0xf6ed)
    }

    /// Cupertino icon named "location_circle". Available on cupertino_icons package 1.0.0+ only.
    pub fn location_circle() -> IconData {
        cupertino(0xf6ef)
    }

    /// Cupertino icon named "location_circle_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn location_circle_fill() -> IconData {
        cupertino(0xf6f0)
    }

    /// Cupertino icon named "location_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn location_fill() -> IconData {
        cupertino(0xf6f1)
    }

    /// Cupertino icon named "location_north". Available on cupertino_icons package 1.0.0+ only.
    pub fn location_north() -> IconData {
        cupertino(0xf6f2)
    }

    /// Cupertino icon named "location_north_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn location_north_fill() -> IconData {
        cupertino(0xf6f3)
    }

    /// Cupertino icon named "location_north_line". Available on cupertino_icons package 1.0.0+ only.
    pub fn location_north_line() -> IconData {
        cupertino(0xf6f4)
    }

    /// Cupertino icon named "location_north_line_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn location_north_line_fill() -> IconData {
        cupertino(0xf6f5)
    }

    /// Cupertino icon named "location_slash". Available on cupertino_icons package 1.0.0+ only.
    pub fn location_slash() -> IconData {
        cupertino(0xf6f6)
    }

    /// Cupertino icon named "location_slash_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn location_slash_fill() -> IconData {
        cupertino(0xf6f7)
    }

    /// Cupertino icon named "lock". Available on cupertino_icons package 1.0.0+ only.
    /// This is the same icon as [`padlock`](Self::padlock()) which is available in cupertino_icons 0.1.3.
    pub fn lock() -> IconData {
        cupertino(0xf4c8)
    }

    /// Cupertino icon named "lock_circle". Available on cupertino_icons package 1.0.0+ only.
    pub fn lock_circle() -> IconData {
        cupertino(0xf6f8)
    }

    /// Cupertino icon named "lock_circle_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn lock_circle_fill() -> IconData {
        cupertino(0xf6f9)
    }

    /// Cupertino icon named "lock_fill". Available on cupertino_icons package 1.0.0+ only.
    /// This is the same icon as [`padlock_solid`](Self::padlock_solid()) which is available in cupertino_icons 0.1.3.
    pub fn lock_fill() -> IconData {
        cupertino(0xf4c9)
    }

    /// Cupertino icon named "lock_open". Available on cupertino_icons package 1.0.0+ only.
    pub fn lock_open() -> IconData {
        cupertino(0xf6fa)
    }

    /// Cupertino icon named "lock_open_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn lock_open_fill() -> IconData {
        cupertino(0xf6fb)
    }

    /// Cupertino icon named "lock_rotation". Available on cupertino_icons package 1.0.0+ only.
    pub fn lock_rotation() -> IconData {
        cupertino(0xf6fc)
    }

    /// Cupertino icon named "lock_rotation_open". Available on cupertino_icons package 1.0.0+ only.
    pub fn lock_rotation_open() -> IconData {
        cupertino(0xf6fd)
    }

    /// Cupertino icon named "lock_shield". Available on cupertino_icons package 1.0.0+ only.
    pub fn lock_shield() -> IconData {
        cupertino(0xf6fe)
    }

    /// Cupertino icon named "lock_shield_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn lock_shield_fill() -> IconData {
        cupertino(0xf6ff)
    }

    /// Cupertino icon named "lock_slash". Available on cupertino_icons package 1.0.0+ only.
    pub fn lock_slash() -> IconData {
        cupertino(0xf700)
    }

    /// Cupertino icon named "lock_slash_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn lock_slash_fill() -> IconData {
        cupertino(0xf701)
    }

    /// Cupertino icon named "macwindow". Available on cupertino_icons package 1.0.0+ only.
    pub fn macwindow() -> IconData {
        cupertino(0xf702)
    }

    /// Cupertino icon named "map". Available on cupertino_icons package 1.0.0+ only.
    pub fn map() -> IconData {
        cupertino(0xf703)
    }

    /// Cupertino icon named "map_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn map_fill() -> IconData {
        cupertino(0xf704)
    }

    /// Cupertino icon named "map_pin". Available on cupertino_icons package 1.0.0+ only.
    pub fn map_pin() -> IconData {
        cupertino(0xf705)
    }

    /// Cupertino icon named "map_pin_ellipse". Available on cupertino_icons package 1.0.0+ only.
    pub fn map_pin_ellipse() -> IconData {
        cupertino(0xf706)
    }

    /// Cupertino icon named "map_pin_slash". Available on cupertino_icons package 1.0.0+ only.
    pub fn map_pin_slash() -> IconData {
        cupertino(0xf707)
    }

    /// Cupertino icon named "memories". Available on cupertino_icons package 1.0.0+ only.
    pub fn memories() -> IconData {
        cupertino(0xf708)
    }

    /// Cupertino icon named "memories_badge_minus". Available on cupertino_icons package 1.0.0+ only.
    pub fn memories_badge_minus() -> IconData {
        cupertino(0xf709)
    }

    /// Cupertino icon named "memories_badge_plus". Available on cupertino_icons package 1.0.0+ only.
    pub fn memories_badge_plus() -> IconData {
        cupertino(0xf70a)
    }

    /// Cupertino icon named "metronome". Available on cupertino_icons package 1.0.0+ only.
    pub fn metronome() -> IconData {
        cupertino(0xf70b)
    }

    /// Cupertino icon named "mic_circle". Available on cupertino_icons package 1.0.0+ only.
    pub fn mic_circle() -> IconData {
        cupertino(0xf70c)
    }

    /// Cupertino icon named "mic_circle_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn mic_circle_fill() -> IconData {
        cupertino(0xf70d)
    }

    /// Cupertino icon named "mic_fill". Available on cupertino_icons package 1.0.0+ only.
    /// This is the same icon as [`mic_solid`](Self::mic_solid()) which is available in cupertino_icons 0.1.3.
    pub fn mic_fill() -> IconData {
        cupertino(0xf461)
    }

    /// Cupertino icon named "mic_slash". Available on cupertino_icons package 1.0.0+ only.
    /// This is the same icon as [`mic_off`](Self::mic_off()) which is available in cupertino_icons 0.1.3.
    pub fn mic_slash() -> IconData {
        cupertino(0xf45f)
    }

    /// Cupertino icon named "mic_slash_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn mic_slash_fill() -> IconData {
        cupertino(0xf70e)
    }

    /// Cupertino icon named "minus". Available on cupertino_icons package 1.0.0+ only.
    pub fn minus() -> IconData {
        cupertino(0xf70f)
    }

    /// Cupertino icon named "minus_circle". Available on cupertino_icons package 1.0.0+ only.
    /// This is the same icon as [`minus_circled`](Self::minus_circled()) which is available in cupertino_icons 0.1.3.
    pub fn minus_circle() -> IconData {
        cupertino(0xf463)
    }

    /// Cupertino icon named "minus_circle_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn minus_circle_fill() -> IconData {
        cupertino(0xf710)
    }

    /// Cupertino icon named "minus_rectangle". Available on cupertino_icons package 1.0.0+ only.
    pub fn minus_rectangle() -> IconData {
        cupertino(0xf711)
    }

    /// Cupertino icon named "minus_rectangle_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn minus_rectangle_fill() -> IconData {
        cupertino(0xf712)
    }

    /// Cupertino icon named "minus_slash_plus". Available on cupertino_icons package 1.0.0+ only.
    pub fn minus_slash_plus() -> IconData {
        cupertino(0xf713)
    }

    /// Cupertino icon named "minus_square". Available on cupertino_icons package 1.0.0+ only.
    pub fn minus_square() -> IconData {
        cupertino(0xf714)
    }

    /// Cupertino icon named "minus_square_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn minus_square_fill() -> IconData {
        cupertino(0xf715)
    }

    /// Cupertino icon named "money_dollar". Available on cupertino_icons package 1.0.0+ only.
    pub fn money_dollar() -> IconData {
        cupertino(0xf8e9)
    }

    /// Cupertino icon named "money_dollar_circle". Available on cupertino_icons package 1.0.0+ only.
    pub fn money_dollar_circle() -> IconData {
        cupertino(0xf8ea)
    }

    /// Cupertino icon named "money_dollar_circle_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn money_dollar_circle_fill() -> IconData {
        cupertino(0xf8eb)
    }

    /// Cupertino icon named "money_euro". Available on cupertino_icons package 1.0.0+ only.
    pub fn money_euro() -> IconData {
        cupertino(0xf8ec)
    }

    /// Cupertino icon named "money_euro_circle". Available on cupertino_icons package 1.0.0+ only.
    pub fn money_euro_circle() -> IconData {
        cupertino(0xf8ed)
    }

    /// Cupertino icon named "money_euro_circle_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn money_euro_circle_fill() -> IconData {
        cupertino(0xf8ee)
    }

    /// Cupertino icon named "money_pound". Available on cupertino_icons package 1.0.0+ only.
    pub fn money_pound() -> IconData {
        cupertino(0xf8ef)
    }

    /// Cupertino icon named "money_pound_circle". Available on cupertino_icons package 1.0.0+ only.
    pub fn money_pound_circle() -> IconData {
        cupertino(0xf8f0)
    }

    /// Cupertino icon named "money_pound_circle_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn money_pound_circle_fill() -> IconData {
        cupertino(0xf8f1)
    }

    /// Cupertino icon named "money_rubl". Available on cupertino_icons package 1.0.0+ only.
    pub fn money_rubl() -> IconData {
        cupertino(0xf8f2)
    }

    /// Cupertino icon named "money_rubl_circle". Available on cupertino_icons package 1.0.0+ only.
    pub fn money_rubl_circle() -> IconData {
        cupertino(0xf8f3)
    }

    /// Cupertino icon named "money_rubl_circle_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn money_rubl_circle_fill() -> IconData {
        cupertino(0xf8f4)
    }

    /// Cupertino icon named "money_yen". Available on cupertino_icons package 1.0.0+ only.
    pub fn money_yen() -> IconData {
        cupertino(0xf8f5)
    }

    /// Cupertino icon named "money_yen_circle". Available on cupertino_icons package 1.0.0+ only.
    pub fn money_yen_circle() -> IconData {
        cupertino(0xf8f6)
    }

    /// Cupertino icon named "money_yen_circle_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn money_yen_circle_fill() -> IconData {
        cupertino(0xf8f7)
    }

    /// Cupertino icon named "moon". Available on cupertino_icons package 1.0.0+ only.
    pub fn moon() -> IconData {
        cupertino(0xf716)
    }

    /// Cupertino icon named "moon_circle". Available on cupertino_icons package 1.0.0+ only.
    pub fn moon_circle() -> IconData {
        cupertino(0xf717)
    }

    /// Cupertino icon named "moon_circle_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn moon_circle_fill() -> IconData {
        cupertino(0xf718)
    }

    /// Cupertino icon named "moon_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn moon_fill() -> IconData {
        cupertino(0xf719)
    }

    /// Cupertino icon named "moon_stars". Available on cupertino_icons package 1.0.0+ only.
    pub fn moon_stars() -> IconData {
        cupertino(0xf71a)
    }

    /// Cupertino icon named "moon_stars_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn moon_stars_fill() -> IconData {
        cupertino(0xf71b)
    }

    /// Cupertino icon named "moon_zzz". Available on cupertino_icons package 1.0.0+ only.
    pub fn moon_zzz() -> IconData {
        cupertino(0xf71c)
    }

    /// Cupertino icon named "moon_zzz_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn moon_zzz_fill() -> IconData {
        cupertino(0xf71d)
    }

    /// Cupertino icon named "move". Available on cupertino_icons package 1.0.0+ only.
    pub fn r#move() -> IconData {
        cupertino(0xf8f8)
    }

    /// Cupertino icon named "multiply". Available on cupertino_icons package 1.0.0+ only.
    pub fn multiply() -> IconData {
        cupertino(0xf71e)
    }

    /// Cupertino icon named "multiply_circle". Available on cupertino_icons package 1.0.0+ only.
    pub fn multiply_circle() -> IconData {
        cupertino(0xf71f)
    }

    /// Cupertino icon named "multiply_circle_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn multiply_circle_fill() -> IconData {
        cupertino(0xf720)
    }

    /// Cupertino icon named "multiply_square". Available on cupertino_icons package 1.0.0+ only.
    pub fn multiply_square() -> IconData {
        cupertino(0xf721)
    }

    /// Cupertino icon named "multiply_square_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn multiply_square_fill() -> IconData {
        cupertino(0xf722)
    }

    /// Cupertino icon named "music_albums". Available on cupertino_icons package 1.0.0+ only.
    pub fn music_albums() -> IconData {
        cupertino(0xf8f9)
    }

    /// Cupertino icon named "music_albums_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn music_albums_fill() -> IconData {
        cupertino(0xf8fa)
    }

    /// Cupertino icon named "music_house". Available on cupertino_icons package 1.0.0+ only.
    pub fn music_house() -> IconData {
        cupertino(0xf723)
    }

    /// Cupertino icon named "music_house_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn music_house_fill() -> IconData {
        cupertino(0xf724)
    }

    /// Cupertino icon named "music_mic". Available on cupertino_icons package 1.0.0+ only.
    pub fn music_mic() -> IconData {
        cupertino(0xf725)
    }

    /// Cupertino icon named "music_note_2". Available on cupertino_icons package 1.0.0+ only.
    /// This is the same icon as [`double_music_note`](Self::double_music_note()) which is available in cupertino_icons 0.1.3.
    pub fn music_note_2() -> IconData {
        cupertino(0xf46c)
    }

    /// Cupertino icon named "music_note_list". Available on cupertino_icons package 1.0.0+ only.
    pub fn music_note_list() -> IconData {
        cupertino(0xf726)
    }

    /// Cupertino icon named "nosign". Available on cupertino_icons package 1.0.0+ only.
    pub fn nosign() -> IconData {
        cupertino(0xf727)
    }

    /// Cupertino icon named "number". Available on cupertino_icons package 1.0.0+ only.
    pub fn number() -> IconData {
        cupertino(0xf728)
    }

    /// Cupertino icon named "number_circle". Available on cupertino_icons package 1.0.0+ only.
    pub fn number_circle() -> IconData {
        cupertino(0xf729)
    }

    /// Cupertino icon named "number_circle_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn number_circle_fill() -> IconData {
        cupertino(0xf72a)
    }

    /// Cupertino icon named "number_square". Available on cupertino_icons package 1.0.0+ only.
    pub fn number_square() -> IconData {
        cupertino(0xf72b)
    }

    /// Cupertino icon named "number_square_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn number_square_fill() -> IconData {
        cupertino(0xf72c)
    }

    /// Cupertino icon named "option". Available on cupertino_icons package 1.0.0+ only.
    pub fn option() -> IconData {
        cupertino(0xf72d)
    }

    /// Cupertino icon named "paintbrush". Available on cupertino_icons package 1.0.0+ only.
    pub fn paintbrush() -> IconData {
        cupertino(0xf72e)
    }

    /// Cupertino icon named "paintbrush_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn paintbrush_fill() -> IconData {
        cupertino(0xf72f)
    }

    /// Cupertino icon named "pano". Available on cupertino_icons package 1.0.0+ only.
    pub fn pano() -> IconData {
        cupertino(0xf730)
    }

    /// Cupertino icon named "pano_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn pano_fill() -> IconData {
        cupertino(0xf731)
    }

    /// Cupertino icon named "paperclip". Available on cupertino_icons package 1.0.0+ only.
    pub fn paperclip() -> IconData {
        cupertino(0xf732)
    }

    /// Cupertino icon named "paperplane". Available on cupertino_icons package 1.0.0+ only.
    pub fn paperplane() -> IconData {
        cupertino(0xf733)
    }

    /// Cupertino icon named "paperplane_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn paperplane_fill() -> IconData {
        cupertino(0xf734)
    }

    /// Cupertino icon named "paragraph". Available on cupertino_icons package 1.0.0+ only.
    pub fn paragraph() -> IconData {
        cupertino(0xf735)
    }

    /// Cupertino icon named "pause_circle". Available on cupertino_icons package 1.0.0+ only.
    pub fn pause_circle() -> IconData {
        cupertino(0xf736)
    }

    /// Cupertino icon named "pause_circle_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn pause_circle_fill() -> IconData {
        cupertino(0xf737)
    }

    /// Cupertino icon named "pause_fill". Available on cupertino_icons package 1.0.0+ only.
    /// This is the same icon as [`pause_solid`](Self::pause_solid()) which is available in cupertino_icons 0.1.3.
    pub fn pause_fill() -> IconData {
        cupertino(0xf478)
    }

    /// Cupertino icon named "pause_rectangle". Available on cupertino_icons package 1.0.0+ only.
    pub fn pause_rectangle() -> IconData {
        cupertino(0xf738)
    }

    /// Cupertino icon named "pause_rectangle_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn pause_rectangle_fill() -> IconData {
        cupertino(0xf739)
    }

    /// Cupertino icon named "pencil_circle". Available on cupertino_icons package 1.0.0+ only.
    pub fn pencil_circle() -> IconData {
        cupertino(0xf73a)
    }

    /// Cupertino icon named "pencil_circle_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn pencil_circle_fill() -> IconData {
        cupertino(0xf73b)
    }

    /// Cupertino icon named "pencil_ellipsis_rectangle". Available on cupertino_icons package 1.0.0+ only.
    pub fn pencil_ellipsis_rectangle() -> IconData {
        cupertino(0xf73c)
    }

    /// Cupertino icon named "pencil_outline". Available on cupertino_icons package 1.0.0+ only.
    pub fn pencil_outline() -> IconData {
        cupertino(0xf73d)
    }

    /// Cupertino icon named "pencil_slash". Available on cupertino_icons package 1.0.0+ only.
    pub fn pencil_slash() -> IconData {
        cupertino(0xf73e)
    }

    /// Cupertino icon named "percent". Available on cupertino_icons package 1.0.0+ only.
    pub fn percent() -> IconData {
        cupertino(0xf73f)
    }

    /// Cupertino icon named "person_2". Available on cupertino_icons package 1.0.0+ only.
    pub fn person_2() -> IconData {
        cupertino(0xf740)
    }

    /// Cupertino icon named "person_2_alt". Available on cupertino_icons package 1.0.0+ only.
    pub fn person_2_alt() -> IconData {
        cupertino(0xf8fb)
    }

    /// Cupertino icon named "person_2_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn person_2_fill() -> IconData {
        cupertino(0xf741)
    }

    /// Cupertino icon named "person_2_square_stack". Available on cupertino_icons package 1.0.0+ only.
    pub fn person_2_square_stack() -> IconData {
        cupertino(0xf742)
    }

    /// Cupertino icon named "person_2_square_stack_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn person_2_square_stack_fill() -> IconData {
        cupertino(0xf743)
    }

    /// Cupertino icon named "person_3". Available on cupertino_icons package 1.0.0+ only.
    /// This is the same icon as [`group`](Self::group()) which is available in cupertino_icons 0.1.3.
    pub fn person_3() -> IconData {
        cupertino(0xf47b)
    }

    /// Cupertino icon named "person_3_fill". Available on cupertino_icons package 1.0.0+ only.
    /// This is the same icon as [`group_solid`](Self::group_solid()) which is available in cupertino_icons 0.1.3.
    pub fn person_3_fill() -> IconData {
        cupertino(0xf47c)
    }

    /// Cupertino icon named "person_alt". Available on cupertino_icons package 1.0.0+ only.
    pub fn person_alt() -> IconData {
        cupertino(0xf8fc)
    }

    /// Cupertino icon named "person_alt_circle". Available on cupertino_icons package 1.0.0+ only.
    pub fn person_alt_circle() -> IconData {
        cupertino(0xf8fd)
    }

    /// Cupertino icon named "person_alt_circle_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn person_alt_circle_fill() -> IconData {
        cupertino(0xf8fe)
    }

    /// Cupertino icon named "person_badge_minus". Available on cupertino_icons package 1.0.0+ only.
    pub fn person_badge_minus() -> IconData {
        cupertino(0xf744)
    }

    /// Cupertino icon named "person_badge_minus_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn person_badge_minus_fill() -> IconData {
        cupertino(0xf745)
    }

    /// Cupertino icon named "person_badge_plus". Available on cupertino_icons package 1.0.0+ only.
    /// This is the same icon as [`person_add`](Self::person_add()) which is available in cupertino_icons 0.1.3.
    pub fn person_badge_plus() -> IconData {
        cupertino(0xf47f)
    }

    /// Cupertino icon named "person_badge_plus_fill". Available on cupertino_icons package 1.0.0+ only.
    /// This is the same icon as [`person_add_solid`](Self::person_add_solid()) which is available in cupertino_icons 0.1.3.
    pub fn person_badge_plus_fill() -> IconData {
        cupertino(0xf480)
    }

    /// Cupertino icon named "person_circle". Available on cupertino_icons package 1.0.0+ only.
    pub fn person_circle() -> IconData {
        cupertino(0xf746)
    }

    /// Cupertino icon named "person_circle_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn person_circle_fill() -> IconData {
        cupertino(0xf747)
    }

    /// Cupertino icon named "person_crop_circle". Available on cupertino_icons package 1.0.0+ only.
    /// This is the same icon as [`profile_circled`](Self::profile_circled()) which is available in cupertino_icons 0.1.3.
    pub fn person_crop_circle() -> IconData {
        cupertino(0xf419)
    }

    /// Cupertino icon named "person_crop_circle_badge_checkmark". Available on cupertino_icons package 1.0.0+ only.
    pub fn person_crop_circle_badge_checkmark() -> IconData {
        cupertino(0xf748)
    }

    /// Cupertino icon named "person_crop_circle_badge_exclam". Available on cupertino_icons package 1.0.0+ only.
    pub fn person_crop_circle_badge_exclam() -> IconData {
        cupertino(0xf749)
    }

    /// Cupertino icon named "person_crop_circle_badge_minus". Available on cupertino_icons package 1.0.0+ only.
    pub fn person_crop_circle_badge_minus() -> IconData {
        cupertino(0xf74a)
    }

    /// Cupertino icon named "person_crop_circle_badge_plus". Available on cupertino_icons package 1.0.0+ only.
    pub fn person_crop_circle_badge_plus() -> IconData {
        cupertino(0xf74b)
    }

    /// Cupertino icon named "person_crop_circle_badge_xmark". Available on cupertino_icons package 1.0.0+ only.
    pub fn person_crop_circle_badge_xmark() -> IconData {
        cupertino(0xf74c)
    }

    /// Cupertino icon named "person_crop_circle_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn person_crop_circle_fill() -> IconData {
        cupertino(0xf74d)
    }

    /// Cupertino icon named "person_crop_circle_fill_badge_checkmark". Available on cupertino_icons package 1.0.0+ only.
    pub fn person_crop_circle_fill_badge_checkmark() -> IconData {
        cupertino(0xf74e)
    }

    /// Cupertino icon named "person_crop_circle_fill_badge_exclam". Available on cupertino_icons package 1.0.0+ only.
    pub fn person_crop_circle_fill_badge_exclam() -> IconData {
        cupertino(0xf74f)
    }

    /// Cupertino icon named "person_crop_circle_fill_badge_minus". Available on cupertino_icons package 1.0.0+ only.
    pub fn person_crop_circle_fill_badge_minus() -> IconData {
        cupertino(0xf750)
    }

    /// Cupertino icon named "person_crop_circle_fill_badge_plus". Available on cupertino_icons package 1.0.0+ only.
    pub fn person_crop_circle_fill_badge_plus() -> IconData {
        cupertino(0xf751)
    }

    /// Cupertino icon named "person_crop_circle_fill_badge_xmark". Available on cupertino_icons package 1.0.0+ only.
    pub fn person_crop_circle_fill_badge_xmark() -> IconData {
        cupertino(0xf752)
    }

    /// Cupertino icon named "person_crop_rectangle". Available on cupertino_icons package 1.0.0+ only.
    pub fn person_crop_rectangle() -> IconData {
        cupertino(0xf753)
    }

    /// Cupertino icon named "person_crop_rectangle_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn person_crop_rectangle_fill() -> IconData {
        cupertino(0xf754)
    }

    /// Cupertino icon named "person_crop_square". Available on cupertino_icons package 1.0.0+ only.
    pub fn person_crop_square() -> IconData {
        cupertino(0xf755)
    }

    /// Cupertino icon named "person_crop_square_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn person_crop_square_fill() -> IconData {
        cupertino(0xf756)
    }

    /// Cupertino icon named "person_fill". Available on cupertino_icons package 1.0.0+ only.
    /// This is the same icon as [`person_solid`](Self::person_solid()) which is available in cupertino_icons 0.1.3.
    pub fn person_fill() -> IconData {
        cupertino(0xf47e)
    }

    /// Cupertino icon named "personalhotspot". Available on cupertino_icons package 1.0.0+ only.
    pub fn personalhotspot() -> IconData {
        cupertino(0xf757)
    }

    /// Cupertino icon named "perspective". Available on cupertino_icons package 1.0.0+ only.
    pub fn perspective() -> IconData {
        cupertino(0xf758)
    }

    /// Cupertino icon named "phone_arrow_down_left". Available on cupertino_icons package 1.0.0+ only.
    pub fn phone_arrow_down_left() -> IconData {
        cupertino(0xf759)
    }

    /// Cupertino icon named "phone_arrow_right". Available on cupertino_icons package 1.0.0+ only.
    pub fn phone_arrow_right() -> IconData {
        cupertino(0xf75a)
    }

    /// Cupertino icon named "phone_arrow_up_right". Available on cupertino_icons package 1.0.0+ only.
    pub fn phone_arrow_up_right() -> IconData {
        cupertino(0xf75b)
    }

    /// Cupertino icon named "phone_badge_plus". Available on cupertino_icons package 1.0.0+ only.
    pub fn phone_badge_plus() -> IconData {
        cupertino(0xf75c)
    }

    /// Cupertino icon named "phone_circle". Available on cupertino_icons package 1.0.0+ only.
    pub fn phone_circle() -> IconData {
        cupertino(0xf75d)
    }

    /// Cupertino icon named "phone_circle_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn phone_circle_fill() -> IconData {
        cupertino(0xf75e)
    }

    /// Cupertino icon named "phone_down". Available on cupertino_icons package 1.0.0+ only.
    pub fn phone_down() -> IconData {
        cupertino(0xf75f)
    }

    /// Cupertino icon named "phone_down_circle". Available on cupertino_icons package 1.0.0+ only.
    pub fn phone_down_circle() -> IconData {
        cupertino(0xf760)
    }

    /// Cupertino icon named "phone_down_circle_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn phone_down_circle_fill() -> IconData {
        cupertino(0xf761)
    }

    /// Cupertino icon named "phone_down_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn phone_down_fill() -> IconData {
        cupertino(0xf762)
    }

    /// Cupertino icon named "phone_fill". Available on cupertino_icons package 1.0.0+ only.
    /// This is the same icon as [`phone_solid`](Self::phone_solid()) which is available in cupertino_icons 0.1.3.
    pub fn phone_fill() -> IconData {
        cupertino(0xf4b9)
    }

    /// Cupertino icon named "phone_fill_arrow_down_left". Available on cupertino_icons package 1.0.0+ only.
    pub fn phone_fill_arrow_down_left() -> IconData {
        cupertino(0xf763)
    }

    /// Cupertino icon named "phone_fill_arrow_right". Available on cupertino_icons package 1.0.0+ only.
    pub fn phone_fill_arrow_right() -> IconData {
        cupertino(0xf764)
    }

    /// Cupertino icon named "phone_fill_arrow_up_right". Available on cupertino_icons package 1.0.0+ only.
    pub fn phone_fill_arrow_up_right() -> IconData {
        cupertino(0xf765)
    }

    /// Cupertino icon named "phone_fill_badge_plus". Available on cupertino_icons package 1.0.0+ only.
    pub fn phone_fill_badge_plus() -> IconData {
        cupertino(0xf766)
    }

    /// Cupertino icon named "photo". Available on cupertino_icons package 1.0.0+ only.
    pub fn photo() -> IconData {
        cupertino(0xf767)
    }

    /// Cupertino icon named "photo_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn photo_fill() -> IconData {
        cupertino(0xf768)
    }

    /// Cupertino icon named "photo_fill_on_rectangle_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn photo_fill_on_rectangle_fill() -> IconData {
        cupertino(0xf769)
    }

    /// Cupertino icon named "photo_on_rectangle". Available on cupertino_icons package 1.0.0+ only.
    pub fn photo_on_rectangle() -> IconData {
        cupertino(0xf76a)
    }

    /// Cupertino icon named "piano". Available on cupertino_icons package 1.0.0+ only.
    pub fn piano() -> IconData {
        cupertino(0xf8ff)
    }

    /// Cupertino icon named "pin". Available on cupertino_icons package 1.0.0+ only.
    pub fn pin() -> IconData {
        cupertino(0xf76b)
    }

    /// Cupertino icon named "pin_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn pin_fill() -> IconData {
        cupertino(0xf76c)
    }

    /// Cupertino icon named "pin_slash". Available on cupertino_icons package 1.0.0+ only.
    pub fn pin_slash() -> IconData {
        cupertino(0xf76d)
    }

    /// Cupertino icon named "pin_slash_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn pin_slash_fill() -> IconData {
        cupertino(0xf76e)
    }

    /// Cupertino icon named "placemark". Available on cupertino_icons package 1.0.0+ only.
    /// This is the same icon as [`location`](Self::location()) which is available in cupertino_icons 0.1.3.
    pub fn placemark() -> IconData {
        cupertino(0xf455)
    }

    /// Cupertino icon named "placemark_fill". Available on cupertino_icons package 1.0.0+ only.
    /// This is the same icon as [`location_solid`](Self::location_solid()) which is available in cupertino_icons 0.1.3.
    pub fn placemark_fill() -> IconData {
        cupertino(0xf456)
    }

    /// Cupertino icon named "play". Available on cupertino_icons package 1.0.0+ only.
    /// This is the same icon as [`play_arrow`](Self::play_arrow()) which is available in cupertino_icons 0.1.3.
    pub fn play() -> IconData {
        cupertino(0xf487)
    }

    /// Cupertino icon named "play_circle". Available on cupertino_icons package 1.0.0+ only.
    pub fn play_circle() -> IconData {
        cupertino(0xf76f)
    }

    /// Cupertino icon named "play_circle_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn play_circle_fill() -> IconData {
        cupertino(0xf770)
    }

    /// Cupertino icon named "play_fill". Available on cupertino_icons package 1.0.0+ only.
    /// This is the same icon as [`play_arrow_solid`](Self::play_arrow_solid()) which is available in cupertino_icons 0.1.3.
    pub fn play_fill() -> IconData {
        cupertino(0xf488)
    }

    /// Cupertino icon named "play_rectangle". Available on cupertino_icons package 1.0.0+ only.
    pub fn play_rectangle() -> IconData {
        cupertino(0xf771)
    }

    /// Cupertino icon named "play_rectangle_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn play_rectangle_fill() -> IconData {
        cupertino(0xf772)
    }

    /// Cupertino icon named "playpause". Available on cupertino_icons package 1.0.0+ only.
    pub fn playpause() -> IconData {
        cupertino(0xf773)
    }

    /// Cupertino icon named "playpause_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn playpause_fill() -> IconData {
        cupertino(0xf774)
    }

    /// Cupertino icon named "plus". Available on cupertino_icons package 1.0.0+ only.
    /// This is the same icon as [`add`](Self::add()) which is available in cupertino_icons 0.1.3.
    pub fn plus() -> IconData {
        cupertino(0xf489)
    }

    /// Cupertino icon named "plus_app". Available on cupertino_icons package 1.0.0+ only.
    pub fn plus_app() -> IconData {
        cupertino(0xf775)
    }

    /// Cupertino icon named "plus_app_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn plus_app_fill() -> IconData {
        cupertino(0xf776)
    }

    /// Cupertino icon named "plus_bubble". Available on cupertino_icons package 1.0.0+ only.
    pub fn plus_bubble() -> IconData {
        cupertino(0xf777)
    }

    /// Cupertino icon named "plus_bubble_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn plus_bubble_fill() -> IconData {
        cupertino(0xf778)
    }

    /// Cupertino icon named "plus_circle". Available on cupertino_icons package 1.0.0+ only.
    /// This is the same icon as [`plus_circled`](Self::plus_circled()) which is available in cupertino_icons 0.1.3.
    /// This is the same icon as [`add_circled`](Self::add_circled()) which is available in cupertino_icons 0.1.3.
    pub fn plus_circle() -> IconData {
        cupertino(0xf48a)
    }

    /// Cupertino icon named "plus_circle_fill". Available on cupertino_icons package 1.0.0+ only.
    /// This is the same icon as [`add_circled_solid`](Self::add_circled_solid()) which is available in cupertino_icons 0.1.3.
    pub fn plus_circle_fill() -> IconData {
        cupertino(0xf48b)
    }

    /// Cupertino icon named "plus_rectangle". Available on cupertino_icons package 1.0.0+ only.
    pub fn plus_rectangle() -> IconData {
        cupertino(0xf779)
    }

    /// Cupertino icon named "plus_rectangle_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn plus_rectangle_fill() -> IconData {
        cupertino(0xf77a)
    }

    /// Cupertino icon named "plus_rectangle_fill_on_rectangle_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn plus_rectangle_fill_on_rectangle_fill() -> IconData {
        cupertino(0xf77b)
    }

    /// Cupertino icon named "plus_rectangle_on_rectangle". Available on cupertino_icons package 1.0.0+ only.
    pub fn plus_rectangle_on_rectangle() -> IconData {
        cupertino(0xf77c)
    }

    /// Cupertino icon named "plus_slash_minus". Available on cupertino_icons package 1.0.0+ only.
    pub fn plus_slash_minus() -> IconData {
        cupertino(0xf77d)
    }

    /// Cupertino icon named "plus_square". Available on cupertino_icons package 1.0.0+ only.
    pub fn plus_square() -> IconData {
        cupertino(0xf77e)
    }

    /// Cupertino icon named "plus_square_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn plus_square_fill() -> IconData {
        cupertino(0xf77f)
    }

    /// Cupertino icon named "plus_square_fill_on_square_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn plus_square_fill_on_square_fill() -> IconData {
        cupertino(0xf780)
    }

    /// Cupertino icon named "plus_square_on_square". Available on cupertino_icons package 1.0.0+ only.
    pub fn plus_square_on_square() -> IconData {
        cupertino(0xf781)
    }

    /// Cupertino icon named "plusminus". Available on cupertino_icons package 1.0.0+ only.
    pub fn plusminus() -> IconData {
        cupertino(0xf782)
    }

    /// Cupertino icon named "plusminus_circle". Available on cupertino_icons package 1.0.0+ only.
    pub fn plusminus_circle() -> IconData {
        cupertino(0xf783)
    }

    /// Cupertino icon named "plusminus_circle_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn plusminus_circle_fill() -> IconData {
        cupertino(0xf784)
    }

    /// Cupertino icon named "power". Available on cupertino_icons package 1.0.0+ only.
    pub fn power() -> IconData {
        cupertino(0xf785)
    }

    /// Cupertino icon named "printer". Available on cupertino_icons package 1.0.0+ only.
    pub fn printer() -> IconData {
        cupertino(0xf786)
    }

    /// Cupertino icon named "printer_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn printer_fill() -> IconData {
        cupertino(0xf787)
    }

    /// Cupertino icon named "projective". Available on cupertino_icons package 1.0.0+ only.
    pub fn projective() -> IconData {
        cupertino(0xf788)
    }

    /// Cupertino icon named "purchased". Available on cupertino_icons package 1.0.0+ only.
    pub fn purchased() -> IconData {
        cupertino(0xf789)
    }

    /// Cupertino icon named "purchased_circle". Available on cupertino_icons package 1.0.0+ only.
    pub fn purchased_circle() -> IconData {
        cupertino(0xf78a)
    }

    /// Cupertino icon named "purchased_circle_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn purchased_circle_fill() -> IconData {
        cupertino(0xf78b)
    }

    /// Cupertino icon named "qrcode". Available on cupertino_icons package 1.0.0+ only.
    pub fn qrcode() -> IconData {
        cupertino(0xf78c)
    }

    /// Cupertino icon named "qrcode_viewfinder". Available on cupertino_icons package 1.0.0+ only.
    pub fn qrcode_viewfinder() -> IconData {
        cupertino(0xf78d)
    }

    /// Cupertino icon named "question". Available on cupertino_icons package 1.0.0+ only.
    pub fn question() -> IconData {
        cupertino(0xf78e)
    }

    /// Cupertino icon named "question_circle". Available on cupertino_icons package 1.0.0+ only.
    pub fn question_circle() -> IconData {
        cupertino(0xf78f)
    }

    /// Cupertino icon named "question_circle_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn question_circle_fill() -> IconData {
        cupertino(0xf790)
    }

    /// Cupertino icon named "question_diamond". Available on cupertino_icons package 1.0.0+ only.
    pub fn question_diamond() -> IconData {
        cupertino(0xf791)
    }

    /// Cupertino icon named "question_diamond_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn question_diamond_fill() -> IconData {
        cupertino(0xf792)
    }

    /// Cupertino icon named "question_square". Available on cupertino_icons package 1.0.0+ only.
    pub fn question_square() -> IconData {
        cupertino(0xf793)
    }

    /// Cupertino icon named "question_square_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn question_square_fill() -> IconData {
        cupertino(0xf794)
    }

    /// Cupertino icon named "quote_bubble". Available on cupertino_icons package 1.0.0+ only.
    pub fn quote_bubble() -> IconData {
        cupertino(0xf795)
    }

    /// Cupertino icon named "quote_bubble_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn quote_bubble_fill() -> IconData {
        cupertino(0xf796)
    }

    /// Cupertino icon named "radiowaves_left". Available on cupertino_icons package 1.0.0+ only.
    pub fn radiowaves_left() -> IconData {
        cupertino(0xf797)
    }

    /// Cupertino icon named "radiowaves_right". Available on cupertino_icons package 1.0.0+ only.
    pub fn radiowaves_right() -> IconData {
        cupertino(0xf798)
    }

    /// Cupertino icon named "rays". Available on cupertino_icons package 1.0.0+ only.
    pub fn rays() -> IconData {
        cupertino(0xf799)
    }

    /// Cupertino icon named "recordingtape". Available on cupertino_icons package 1.0.0+ only.
    pub fn recordingtape() -> IconData {
        cupertino(0xf79a)
    }

    /// Cupertino icon named "rectangle". Available on cupertino_icons package 1.0.0+ only.
    pub fn rectangle() -> IconData {
        cupertino(0xf79b)
    }

    /// Cupertino icon named "rectangle_3_offgrid". Available on cupertino_icons package 1.0.0+ only.
    pub fn rectangle_3_offgrid() -> IconData {
        cupertino(0xf79c)
    }

    /// Cupertino icon named "rectangle_3_offgrid_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn rectangle_3_offgrid_fill() -> IconData {
        cupertino(0xf79d)
    }

    /// Cupertino icon named "rectangle_arrow_up_right_arrow_down_left". Available on cupertino_icons package 1.0.0+ only.
    pub fn rectangle_arrow_up_right_arrow_down_left() -> IconData {
        cupertino(0xf79e)
    }

    /// Cupertino icon named "rectangle_arrow_up_right_arrow_down_left_slash". Available on cupertino_icons package 1.0.0+ only.
    pub fn rectangle_arrow_up_right_arrow_down_left_slash() -> IconData {
        cupertino(0xf79f)
    }

    /// Cupertino icon named "rectangle_badge_checkmark". Available on cupertino_icons package 1.0.0+ only.
    pub fn rectangle_badge_checkmark() -> IconData {
        cupertino(0xf7a0)
    }

    /// Cupertino icon named "rectangle_badge_xmark". Available on cupertino_icons package 1.0.0+ only.
    pub fn rectangle_badge_xmark() -> IconData {
        cupertino(0xf7a1)
    }

    /// Cupertino icon named "rectangle_compress_vertical". Available on cupertino_icons package 1.0.0+ only.
    pub fn rectangle_compress_vertical() -> IconData {
        cupertino(0xf7a2)
    }

    /// Cupertino icon named "rectangle_dock". Available on cupertino_icons package 1.0.0+ only.
    pub fn rectangle_dock() -> IconData {
        cupertino(0xf7a3)
    }

    /// Cupertino icon named "rectangle_expand_vertical". Available on cupertino_icons package 1.0.0+ only.
    pub fn rectangle_expand_vertical() -> IconData {
        cupertino(0xf7a4)
    }

    /// Cupertino icon named "rectangle_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn rectangle_fill() -> IconData {
        cupertino(0xf7a5)
    }

    /// Cupertino icon named "rectangle_fill_badge_checkmark". Available on cupertino_icons package 1.0.0+ only.
    pub fn rectangle_fill_badge_checkmark() -> IconData {
        cupertino(0xf7a6)
    }

    /// Cupertino icon named "rectangle_fill_badge_xmark". Available on cupertino_icons package 1.0.0+ only.
    pub fn rectangle_fill_badge_xmark() -> IconData {
        cupertino(0xf7a7)
    }

    /// Cupertino icon named "rectangle_fill_on_rectangle_angled_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn rectangle_fill_on_rectangle_angled_fill() -> IconData {
        cupertino(0xf7a8)
    }

    /// Cupertino icon named "rectangle_fill_on_rectangle_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn rectangle_fill_on_rectangle_fill() -> IconData {
        cupertino(0xf7a9)
    }

    /// Cupertino icon named "rectangle_grid_1x2". Available on cupertino_icons package 1.0.0+ only.
    pub fn rectangle_grid_1x2() -> IconData {
        cupertino(0xf7aa)
    }

    /// Cupertino icon named "rectangle_grid_1x2_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn rectangle_grid_1x2_fill() -> IconData {
        cupertino(0xf7ab)
    }

    /// Cupertino icon named "rectangle_grid_2x2". Available on cupertino_icons package 1.0.0+ only.
    pub fn rectangle_grid_2x2() -> IconData {
        cupertino(0xf7ac)
    }

    /// Cupertino icon named "rectangle_grid_2x2_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn rectangle_grid_2x2_fill() -> IconData {
        cupertino(0xf7ad)
    }

    /// Cupertino icon named "rectangle_grid_3x2". Available on cupertino_icons package 1.0.0+ only.
    pub fn rectangle_grid_3x2() -> IconData {
        cupertino(0xf7ae)
    }

    /// Cupertino icon named "rectangle_grid_3x2_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn rectangle_grid_3x2_fill() -> IconData {
        cupertino(0xf7af)
    }

    /// Cupertino icon named "rectangle_on_rectangle". Available on cupertino_icons package 1.0.0+ only.
    pub fn rectangle_on_rectangle() -> IconData {
        cupertino(0xf7b0)
    }

    /// Cupertino icon named "rectangle_on_rectangle_angled". Available on cupertino_icons package 1.0.0+ only.
    pub fn rectangle_on_rectangle_angled() -> IconData {
        cupertino(0xf7b1)
    }

    /// Cupertino icon named "rectangle_paperclip". Available on cupertino_icons package 1.0.0+ only.
    pub fn rectangle_paperclip() -> IconData {
        cupertino(0xf7b2)
    }

    /// Cupertino icon named "rectangle_split_3x1". Available on cupertino_icons package 1.0.0+ only.
    pub fn rectangle_split_3x1() -> IconData {
        cupertino(0xf7b3)
    }

    /// Cupertino icon named "rectangle_split_3x1_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn rectangle_split_3x1_fill() -> IconData {
        cupertino(0xf7b4)
    }

    /// Cupertino icon named "rectangle_split_3x3". Available on cupertino_icons package 1.0.0+ only.
    pub fn rectangle_split_3x3() -> IconData {
        cupertino(0xf7b5)
    }

    /// Cupertino icon named "rectangle_split_3x3_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn rectangle_split_3x3_fill() -> IconData {
        cupertino(0xf7b6)
    }

    /// Cupertino icon named "rectangle_stack". Available on cupertino_icons package 1.0.0+ only.
    /// This is the same icon as [`collections`](Self::collections()) which is available in cupertino_icons 0.1.3.
    pub fn rectangle_stack() -> IconData {
        cupertino(0xf3c9)
    }

    /// Cupertino icon named "rectangle_stack_badge_minus". Available on cupertino_icons package 1.0.0+ only.
    pub fn rectangle_stack_badge_minus() -> IconData {
        cupertino(0xf7b7)
    }

    /// Cupertino icon named "rectangle_stack_badge_person_crop". Available on cupertino_icons package 1.0.0+ only.
    pub fn rectangle_stack_badge_person_crop() -> IconData {
        cupertino(0xf7b8)
    }

    /// Cupertino icon named "rectangle_stack_badge_plus". Available on cupertino_icons package 1.0.0+ only.
    pub fn rectangle_stack_badge_plus() -> IconData {
        cupertino(0xf7b9)
    }

    /// Cupertino icon named "rectangle_stack_fill". Available on cupertino_icons package 1.0.0+ only.
    /// This is the same icon as [`collections_solid`](Self::collections_solid()) which is available in cupertino_icons 0.1.3.
    pub fn rectangle_stack_fill() -> IconData {
        cupertino(0xf3ca)
    }

    /// Cupertino icon named "rectangle_stack_fill_badge_minus". Available on cupertino_icons package 1.0.0+ only.
    pub fn rectangle_stack_fill_badge_minus() -> IconData {
        cupertino(0xf7ba)
    }

    /// Cupertino icon named "rectangle_stack_fill_badge_person_crop". Available on cupertino_icons package 1.0.0+ only.
    pub fn rectangle_stack_fill_badge_person_crop() -> IconData {
        cupertino(0xf7bb)
    }

    /// Cupertino icon named "rectangle_stack_fill_badge_plus". Available on cupertino_icons package 1.0.0+ only.
    pub fn rectangle_stack_fill_badge_plus() -> IconData {
        cupertino(0xf7bc)
    }

    /// Cupertino icon named "rectangle_stack_person_crop". Available on cupertino_icons package 1.0.0+ only.
    pub fn rectangle_stack_person_crop() -> IconData {
        cupertino(0xf7bd)
    }

    /// Cupertino icon named "rectangle_stack_person_crop_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn rectangle_stack_person_crop_fill() -> IconData {
        cupertino(0xf7be)
    }

    /// Cupertino icon named "repeat". Available on cupertino_icons package 1.0.0+ only.
    pub fn repeat() -> IconData {
        cupertino(0xf7bf)
    }

    /// Cupertino icon named "repeat_1". Available on cupertino_icons package 1.0.0+ only.
    pub fn repeat_1() -> IconData {
        cupertino(0xf7c0)
    }

    /// Cupertino icon named "resize". Available on cupertino_icons package 1.0.0+ only.
    pub fn resize() -> IconData {
        cupertino(0xf900)
    }

    /// Cupertino icon named "resize_h". Available on cupertino_icons package 1.0.0+ only.
    pub fn resize_h() -> IconData {
        cupertino(0xf901)
    }

    /// Cupertino icon named "resize_v". Available on cupertino_icons package 1.0.0+ only.
    pub fn resize_v() -> IconData {
        cupertino(0xf902)
    }

    /// Cupertino icon named "return_icon". Available on cupertino_icons package 1.0.0+ only.
    pub fn return_icon() -> IconData {
        cupertino(0xf7c1)
    }

    /// Cupertino icon named "rhombus". Available on cupertino_icons package 1.0.0+ only.
    pub fn rhombus() -> IconData {
        cupertino(0xf7c2)
    }

    /// Cupertino icon named "rhombus_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn rhombus_fill() -> IconData {
        cupertino(0xf7c3)
    }

    /// Cupertino icon named "rocket". Available on cupertino_icons package 1.0.0+ only.
    pub fn rocket() -> IconData {
        cupertino(0xf903)
    }

    /// Cupertino icon named "rocket_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn rocket_fill() -> IconData {
        cupertino(0xf904)
    }

    /// Cupertino icon named "rosette". Available on cupertino_icons package 1.0.0+ only.
    pub fn rosette() -> IconData {
        cupertino(0xf7c4)
    }

    /// Cupertino icon named "rotate_left". Available on cupertino_icons package 1.0.0+ only.
    pub fn rotate_left() -> IconData {
        cupertino(0xf7c5)
    }

    /// Cupertino icon named "rotate_left_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn rotate_left_fill() -> IconData {
        cupertino(0xf7c6)
    }

    /// Cupertino icon named "rotate_right". Available on cupertino_icons package 1.0.0+ only.
    pub fn rotate_right() -> IconData {
        cupertino(0xf7c7)
    }

    /// Cupertino icon named "rotate_right_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn rotate_right_fill() -> IconData {
        cupertino(0xf7c8)
    }

    /// Cupertino icon named "scissors". Available on cupertino_icons package 1.0.0+ only.
    pub fn scissors() -> IconData {
        cupertino(0xf7c9)
    }

    /// Cupertino icon named "scissors_alt". Available on cupertino_icons package 1.0.0+ only.
    pub fn scissors_alt() -> IconData {
        cupertino(0xf905)
    }

    /// Cupertino icon named "scope". Available on cupertino_icons package 1.0.0+ only.
    pub fn scope() -> IconData {
        cupertino(0xf7ca)
    }

    /// Cupertino icon named "scribble". Available on cupertino_icons package 1.0.0+ only.
    pub fn scribble() -> IconData {
        cupertino(0xf7cb)
    }

    /// Cupertino icon named "search_circle". Available on cupertino_icons package 1.0.0+ only.
    pub fn search_circle() -> IconData {
        cupertino(0xf7cc)
    }

    /// Cupertino icon named "search_circle_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn search_circle_fill() -> IconData {
        cupertino(0xf7cd)
    }

    /// Cupertino icon named "selection_pin_in_out". Available on cupertino_icons package 1.0.0+ only.
    pub fn selection_pin_in_out() -> IconData {
        cupertino(0xf7ce)
    }

    /// Cupertino icon named "shield". Available on cupertino_icons package 1.0.0+ only.
    pub fn shield() -> IconData {
        cupertino(0xf7cf)
    }

    /// Cupertino icon named "shield_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn shield_fill() -> IconData {
        cupertino(0xf7d0)
    }

    /// Cupertino icon named "shield_lefthalf_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn shield_lefthalf_fill() -> IconData {
        cupertino(0xf7d1)
    }

    /// Cupertino icon named "shield_slash". Available on cupertino_icons package 1.0.0+ only.
    pub fn shield_slash() -> IconData {
        cupertino(0xf7d2)
    }

    /// Cupertino icon named "shield_slash_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn shield_slash_fill() -> IconData {
        cupertino(0xf7d3)
    }

    /// Cupertino icon named "shift". Available on cupertino_icons package 1.0.0+ only.
    pub fn shift() -> IconData {
        cupertino(0xf7d4)
    }

    /// Cupertino icon named "shift_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn shift_fill() -> IconData {
        cupertino(0xf7d5)
    }

    /// Cupertino icon named "sidebar_left". Available on cupertino_icons package 1.0.0+ only.
    pub fn sidebar_left() -> IconData {
        cupertino(0xf7d6)
    }

    /// Cupertino icon named "sidebar_right". Available on cupertino_icons package 1.0.0+ only.
    pub fn sidebar_right() -> IconData {
        cupertino(0xf7d7)
    }

    /// Cupertino icon named "signature". Available on cupertino_icons package 1.0.0+ only.
    pub fn signature() -> IconData {
        cupertino(0xf7d8)
    }

    /// Cupertino icon named "skew". Available on cupertino_icons package 1.0.0+ only.
    pub fn skew() -> IconData {
        cupertino(0xf7d9)
    }

    /// Cupertino icon named "slash_circle". Available on cupertino_icons package 1.0.0+ only.
    pub fn slash_circle() -> IconData {
        cupertino(0xf7da)
    }

    /// Cupertino icon named "slash_circle_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn slash_circle_fill() -> IconData {
        cupertino(0xf7db)
    }

    /// Cupertino icon named "slider_horizontal_3". Available on cupertino_icons package 1.0.0+ only.
    pub fn slider_horizontal_3() -> IconData {
        cupertino(0xf7dc)
    }

    /// Cupertino icon named "slider_horizontal_below_rectangle". Available on cupertino_icons package 1.0.0+ only.
    pub fn slider_horizontal_below_rectangle() -> IconData {
        cupertino(0xf7dd)
    }

    /// Cupertino icon named "slowmo". Available on cupertino_icons package 1.0.0+ only.
    pub fn slowmo() -> IconData {
        cupertino(0xf7de)
    }

    /// Cupertino icon named "smallcircle_circle". Available on cupertino_icons package 1.0.0+ only.
    pub fn smallcircle_circle() -> IconData {
        cupertino(0xf7df)
    }

    /// Cupertino icon named "smallcircle_circle_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn smallcircle_circle_fill() -> IconData {
        cupertino(0xf7e0)
    }

    /// Cupertino icon named "smallcircle_fill_circle". Available on cupertino_icons package 1.0.0+ only.
    pub fn smallcircle_fill_circle() -> IconData {
        cupertino(0xf7e1)
    }

    /// Cupertino icon named "smallcircle_fill_circle_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn smallcircle_fill_circle_fill() -> IconData {
        cupertino(0xf7e2)
    }

    /// Cupertino icon named "smiley". Available on cupertino_icons package 1.0.0+ only.
    pub fn smiley() -> IconData {
        cupertino(0xf7e3)
    }

    /// Cupertino icon named "smiley_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn smiley_fill() -> IconData {
        cupertino(0xf7e4)
    }

    /// Cupertino icon named "smoke". Available on cupertino_icons package 1.0.0+ only.
    pub fn smoke() -> IconData {
        cupertino(0xf7e5)
    }

    /// Cupertino icon named "smoke_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn smoke_fill() -> IconData {
        cupertino(0xf7e6)
    }

    /// Cupertino icon named "snow". Available on cupertino_icons package 1.0.0+ only.
    pub fn snow() -> IconData {
        cupertino(0xf7e7)
    }

    /// Cupertino icon named "sort_down". Available on cupertino_icons package 1.0.0+ only.
    pub fn sort_down() -> IconData {
        cupertino(0xf906)
    }

    /// Cupertino icon named "sort_down_circle". Available on cupertino_icons package 1.0.0+ only.
    pub fn sort_down_circle() -> IconData {
        cupertino(0xf907)
    }

    /// Cupertino icon named "sort_down_circle_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn sort_down_circle_fill() -> IconData {
        cupertino(0xf908)
    }

    /// Cupertino icon named "sort_up". Available on cupertino_icons package 1.0.0+ only.
    pub fn sort_up() -> IconData {
        cupertino(0xf909)
    }

    /// Cupertino icon named "sort_up_circle". Available on cupertino_icons package 1.0.0+ only.
    pub fn sort_up_circle() -> IconData {
        cupertino(0xf90a)
    }

    /// Cupertino icon named "sort_up_circle_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn sort_up_circle_fill() -> IconData {
        cupertino(0xf90b)
    }

    /// Cupertino icon named "sparkles". Available on cupertino_icons package 1.0.0+ only.
    pub fn sparkles() -> IconData {
        cupertino(0xf7e8)
    }

    /// Cupertino icon named "speaker". Available on cupertino_icons package 1.0.0+ only.
    pub fn speaker() -> IconData {
        cupertino(0xf7e9)
    }

    /// Cupertino icon named "speaker_1". Available on cupertino_icons package 1.0.0+ only.
    pub fn speaker_1() -> IconData {
        cupertino(0xf7ea)
    }

    /// Cupertino icon named "speaker_1_fill". Available on cupertino_icons package 1.0.0+ only.
    /// This is the same icon as [`volume_down`](Self::volume_down()) which is available in cupertino_icons 0.1.3.
    pub fn speaker_1_fill() -> IconData {
        cupertino(0xf3b7)
    }

    /// Cupertino icon named "speaker_2". Available on cupertino_icons package 1.0.0+ only.
    pub fn speaker_2() -> IconData {
        cupertino(0xf7eb)
    }

    /// Cupertino icon named "speaker_2_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn speaker_2_fill() -> IconData {
        cupertino(0xf7ec)
    }

    /// Cupertino icon named "speaker_3". Available on cupertino_icons package 1.0.0+ only.
    pub fn speaker_3() -> IconData {
        cupertino(0xf7ed)
    }

    /// Cupertino icon named "speaker_3_fill". Available on cupertino_icons package 1.0.0+ only.
    /// This is the same icon as [`volume_up`](Self::volume_up()) which is available in cupertino_icons 0.1.3.
    pub fn speaker_3_fill() -> IconData {
        cupertino(0xf3ba)
    }

    /// Cupertino icon named "speaker_fill". Available on cupertino_icons package 1.0.0+ only.
    /// This is the same icon as [`volume_mute`](Self::volume_mute()) which is available in cupertino_icons 0.1.3.
    pub fn speaker_fill() -> IconData {
        cupertino(0xf3b8)
    }

    /// Cupertino icon named "speaker_slash". Available on cupertino_icons package 1.0.0+ only.
    pub fn speaker_slash() -> IconData {
        cupertino(0xf7ee)
    }

    /// Cupertino icon named "speaker_slash_fill". Available on cupertino_icons package 1.0.0+ only.
    /// This is the same icon as [`volume_off`](Self::volume_off()) which is available in cupertino_icons 0.1.3.
    pub fn speaker_slash_fill() -> IconData {
        cupertino(0xf3b9)
    }

    /// Cupertino icon named "speaker_slash_fill_rtl". Available on cupertino_icons package 1.0.0+ only.
    pub fn speaker_slash_fill_rtl() -> IconData {
        cupertino(0xf7ef)
    }

    /// Cupertino icon named "speaker_slash_rtl". Available on cupertino_icons package 1.0.0+ only.
    pub fn speaker_slash_rtl() -> IconData {
        cupertino(0xf7f0)
    }

    /// Cupertino icon named "speaker_zzz". Available on cupertino_icons package 1.0.0+ only.
    pub fn speaker_zzz() -> IconData {
        cupertino(0xf7f1)
    }

    /// Cupertino icon named "speaker_zzz_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn speaker_zzz_fill() -> IconData {
        cupertino(0xf7f2)
    }

    /// Cupertino icon named "speaker_zzz_fill_rtl". Available on cupertino_icons package 1.0.0+ only.
    pub fn speaker_zzz_fill_rtl() -> IconData {
        cupertino(0xf7f3)
    }

    /// Cupertino icon named "speaker_zzz_rtl". Available on cupertino_icons package 1.0.0+ only.
    pub fn speaker_zzz_rtl() -> IconData {
        cupertino(0xf7f4)
    }

    /// Cupertino icon named "speedometer". Available on cupertino_icons package 1.0.0+ only.
    pub fn speedometer() -> IconData {
        cupertino(0xf7f5)
    }

    /// Cupertino icon named "sportscourt". Available on cupertino_icons package 1.0.0+ only.
    pub fn sportscourt() -> IconData {
        cupertino(0xf7f6)
    }

    /// Cupertino icon named "sportscourt_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn sportscourt_fill() -> IconData {
        cupertino(0xf7f7)
    }

    /// Cupertino icon named "square". Available on cupertino_icons package 1.0.0+ only.
    pub fn square() -> IconData {
        cupertino(0xf7f8)
    }

    /// Cupertino icon named "square_arrow_down". Available on cupertino_icons package 1.0.0+ only.
    pub fn square_arrow_down() -> IconData {
        cupertino(0xf7f9)
    }

    /// Cupertino icon named "square_arrow_down_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn square_arrow_down_fill() -> IconData {
        cupertino(0xf7fa)
    }

    /// Cupertino icon named "square_arrow_down_on_square". Available on cupertino_icons package 1.0.0+ only.
    pub fn square_arrow_down_on_square() -> IconData {
        cupertino(0xf7fb)
    }

    /// Cupertino icon named "square_arrow_down_on_square_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn square_arrow_down_on_square_fill() -> IconData {
        cupertino(0xf7fc)
    }

    /// Cupertino icon named "square_arrow_left". Available on cupertino_icons package 1.0.0+ only.
    pub fn square_arrow_left() -> IconData {
        cupertino(0xf90c)
    }

    /// Cupertino icon named "square_arrow_left_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn square_arrow_left_fill() -> IconData {
        cupertino(0xf90d)
    }

    /// Cupertino icon named "square_arrow_right". Available on cupertino_icons package 1.0.0+ only.
    pub fn square_arrow_right() -> IconData {
        cupertino(0xf90e)
    }

    /// Cupertino icon named "square_arrow_right_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn square_arrow_right_fill() -> IconData {
        cupertino(0xf90f)
    }

    /// Cupertino icon named "square_arrow_up". Available on cupertino_icons package 1.0.0+ only.
    /// This is the same icon as [`share`](Self::share()) which is available in cupertino_icons 0.1.3.
    /// This is the same icon as [`share_up`](Self::share_up()) which is available in cupertino_icons 0.1.3.
    pub fn square_arrow_up() -> IconData {
        cupertino(0xf4ca)
    }

    /// Cupertino icon named "square_arrow_up_fill". Available on cupertino_icons package 1.0.0+ only.
    /// This is the same icon as [`share_solid`](Self::share_solid()) which is available in cupertino_icons 0.1.3.
    pub fn square_arrow_up_fill() -> IconData {
        cupertino(0xf4cb)
    }

    /// Cupertino icon named "square_arrow_up_on_square". Available on cupertino_icons package 1.0.0+ only.
    pub fn square_arrow_up_on_square() -> IconData {
        cupertino(0xf7fd)
    }

    /// Cupertino icon named "square_arrow_up_on_square_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn square_arrow_up_on_square_fill() -> IconData {
        cupertino(0xf7fe)
    }

    /// Cupertino icon named "square_favorites". Available on cupertino_icons package 1.0.0+ only.
    pub fn square_favorites() -> IconData {
        cupertino(0xf910)
    }

    /// Cupertino icon named "square_favorites_alt". Available on cupertino_icons package 1.0.0+ only.
    pub fn square_favorites_alt() -> IconData {
        cupertino(0xf911)
    }

    /// Cupertino icon named "square_favorites_alt_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn square_favorites_alt_fill() -> IconData {
        cupertino(0xf912)
    }

    /// Cupertino icon named "square_favorites_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn square_favorites_fill() -> IconData {
        cupertino(0xf913)
    }

    /// Cupertino icon named "square_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn square_fill() -> IconData {
        cupertino(0xf7ff)
    }

    /// Cupertino icon named "square_fill_line_vertical_square". Available on cupertino_icons package 1.0.0+ only.
    pub fn square_fill_line_vertical_square() -> IconData {
        cupertino(0xf800)
    }

    /// Cupertino icon named "square_fill_line_vertical_square_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn square_fill_line_vertical_square_fill() -> IconData {
        cupertino(0xf801)
    }

    /// Cupertino icon named "square_fill_on_circle_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn square_fill_on_circle_fill() -> IconData {
        cupertino(0xf802)
    }

    /// Cupertino icon named "square_fill_on_square_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn square_fill_on_square_fill() -> IconData {
        cupertino(0xf803)
    }

    /// Cupertino icon named "square_grid_2x2". Available on cupertino_icons package 1.0.0+ only.
    pub fn square_grid_2x2() -> IconData {
        cupertino(0xf804)
    }

    /// Cupertino icon named "square_grid_2x2_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn square_grid_2x2_fill() -> IconData {
        cupertino(0xf805)
    }

    /// Cupertino icon named "square_grid_3x2". Available on cupertino_icons package 1.0.0+ only.
    pub fn square_grid_3x2() -> IconData {
        cupertino(0xf806)
    }

    /// Cupertino icon named "square_grid_3x2_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn square_grid_3x2_fill() -> IconData {
        cupertino(0xf807)
    }

    /// Cupertino icon named "square_grid_4x3_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn square_grid_4x3_fill() -> IconData {
        cupertino(0xf808)
    }

    /// Cupertino icon named "square_lefthalf_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn square_lefthalf_fill() -> IconData {
        cupertino(0xf809)
    }

    /// Cupertino icon named "square_line_vertical_square". Available on cupertino_icons package 1.0.0+ only.
    pub fn square_line_vertical_square() -> IconData {
        cupertino(0xf80a)
    }

    /// Cupertino icon named "square_line_vertical_square_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn square_line_vertical_square_fill() -> IconData {
        cupertino(0xf80b)
    }

    /// Cupertino icon named "square_list". Available on cupertino_icons package 1.0.0+ only.
    pub fn square_list() -> IconData {
        cupertino(0xf914)
    }

    /// Cupertino icon named "square_list_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn square_list_fill() -> IconData {
        cupertino(0xf915)
    }

    /// Cupertino icon named "square_on_circle". Available on cupertino_icons package 1.0.0+ only.
    pub fn square_on_circle() -> IconData {
        cupertino(0xf80c)
    }

    /// Cupertino icon named "square_on_square". Available on cupertino_icons package 1.0.0+ only.
    pub fn square_on_square() -> IconData {
        cupertino(0xf80d)
    }

    /// Cupertino icon named "square_pencil". Available on cupertino_icons package 1.0.0+ only.
    /// This is the same icon as [`create`](Self::create()) which is available in cupertino_icons 0.1.3.
    /// This is the same icon as [`create_solid`](Self::create_solid()) which is available in cupertino_icons 0.1.3.
    pub fn square_pencil() -> IconData {
        cupertino(0xf417)
    }

    /// Cupertino icon named "square_pencil_fill". Available on cupertino_icons package 1.0.0+ only.
    /// This is the same icon as [`create`](Self::create()) which is available in cupertino_icons 0.1.3.
    /// This is the same icon as [`create_solid`](Self::create_solid()) which is available in cupertino_icons 0.1.3.
    pub fn square_pencil_fill() -> IconData {
        cupertino(0xf417)
    }

    /// Cupertino icon named "square_righthalf_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn square_righthalf_fill() -> IconData {
        cupertino(0xf80e)
    }

    /// Cupertino icon named "square_split_1x2". Available on cupertino_icons package 1.0.0+ only.
    pub fn square_split_1x2() -> IconData {
        cupertino(0xf80f)
    }

    /// Cupertino icon named "square_split_1x2_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn square_split_1x2_fill() -> IconData {
        cupertino(0xf810)
    }

    /// Cupertino icon named "square_split_2x1". Available on cupertino_icons package 1.0.0+ only.
    pub fn square_split_2x1() -> IconData {
        cupertino(0xf811)
    }

    /// Cupertino icon named "square_split_2x1_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn square_split_2x1_fill() -> IconData {
        cupertino(0xf812)
    }

    /// Cupertino icon named "square_split_2x2". Available on cupertino_icons package 1.0.0+ only.
    pub fn square_split_2x2() -> IconData {
        cupertino(0xf813)
    }

    /// Cupertino icon named "square_split_2x2_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn square_split_2x2_fill() -> IconData {
        cupertino(0xf814)
    }

    /// Cupertino icon named "square_stack". Available on cupertino_icons package 1.0.0+ only.
    pub fn square_stack() -> IconData {
        cupertino(0xf815)
    }

    /// Cupertino icon named "square_stack_3d_down_dottedline". Available on cupertino_icons package 1.0.0+ only.
    pub fn square_stack_3d_down_dottedline() -> IconData {
        cupertino(0xf816)
    }

    /// Cupertino icon named "square_stack_3d_down_right". Available on cupertino_icons package 1.0.0+ only.
    pub fn square_stack_3d_down_right() -> IconData {
        cupertino(0xf817)
    }

    /// Cupertino icon named "square_stack_3d_down_right_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn square_stack_3d_down_right_fill() -> IconData {
        cupertino(0xf818)
    }

    /// Cupertino icon named "square_stack_3d_up". Available on cupertino_icons package 1.0.0+ only.
    pub fn square_stack_3d_up() -> IconData {
        cupertino(0xf819)
    }

    /// Cupertino icon named "square_stack_3d_up_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn square_stack_3d_up_fill() -> IconData {
        cupertino(0xf81a)
    }

    /// Cupertino icon named "square_stack_3d_up_slash". Available on cupertino_icons package 1.0.0+ only.
    pub fn square_stack_3d_up_slash() -> IconData {
        cupertino(0xf81b)
    }

    /// Cupertino icon named "square_stack_3d_up_slash_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn square_stack_3d_up_slash_fill() -> IconData {
        cupertino(0xf81c)
    }

    /// Cupertino icon named "square_stack_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn square_stack_fill() -> IconData {
        cupertino(0xf81d)
    }

    /// Cupertino icon named "squares_below_rectangle". Available on cupertino_icons package 1.0.0+ only.
    pub fn squares_below_rectangle() -> IconData {
        cupertino(0xf81e)
    }

    /// Cupertino icon named "star". Available on cupertino_icons package 1.0.0+ only.
    pub fn star() -> IconData {
        cupertino(0xf81f)
    }

    /// Cupertino icon named "star_circle". Available on cupertino_icons package 1.0.0+ only.
    pub fn star_circle() -> IconData {
        cupertino(0xf820)
    }

    /// Cupertino icon named "star_circle_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn star_circle_fill() -> IconData {
        cupertino(0xf821)
    }

    /// Cupertino icon named "star_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn star_fill() -> IconData {
        cupertino(0xf822)
    }

    /// Cupertino icon named "star_lefthalf_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn star_lefthalf_fill() -> IconData {
        cupertino(0xf823)
    }

    /// Cupertino icon named "star_slash". Available on cupertino_icons package 1.0.0+ only.
    pub fn star_slash() -> IconData {
        cupertino(0xf824)
    }

    /// Cupertino icon named "star_slash_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn star_slash_fill() -> IconData {
        cupertino(0xf825)
    }

    /// Cupertino icon named "staroflife". Available on cupertino_icons package 1.0.0+ only.
    pub fn staroflife() -> IconData {
        cupertino(0xf826)
    }

    /// Cupertino icon named "staroflife_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn staroflife_fill() -> IconData {
        cupertino(0xf827)
    }

    /// Cupertino icon named "stop". Available on cupertino_icons package 1.0.0+ only.
    pub fn stop() -> IconData {
        cupertino(0xf828)
    }

    /// Cupertino icon named "stop_circle". Available on cupertino_icons package 1.0.0+ only.
    pub fn stop_circle() -> IconData {
        cupertino(0xf829)
    }

    /// Cupertino icon named "stop_circle_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn stop_circle_fill() -> IconData {
        cupertino(0xf82a)
    }

    /// Cupertino icon named "stop_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn stop_fill() -> IconData {
        cupertino(0xf82b)
    }

    /// Cupertino icon named "stopwatch". Available on cupertino_icons package 1.0.0+ only.
    pub fn stopwatch() -> IconData {
        cupertino(0xf82c)
    }

    /// Cupertino icon named "stopwatch_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn stopwatch_fill() -> IconData {
        cupertino(0xf82d)
    }

    /// Cupertino icon named "strikethrough". Available on cupertino_icons package 1.0.0+ only.
    pub fn strikethrough() -> IconData {
        cupertino(0xf82e)
    }

    /// Cupertino icon named "suit_club". Available on cupertino_icons package 1.0.0+ only.
    pub fn suit_club() -> IconData {
        cupertino(0xf82f)
    }

    /// Cupertino icon named "suit_club_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn suit_club_fill() -> IconData {
        cupertino(0xf830)
    }

    /// Cupertino icon named "suit_diamond". Available on cupertino_icons package 1.0.0+ only.
    pub fn suit_diamond() -> IconData {
        cupertino(0xf831)
    }

    /// Cupertino icon named "suit_diamond_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn suit_diamond_fill() -> IconData {
        cupertino(0xf832)
    }

    /// Cupertino icon named "suit_heart". Available on cupertino_icons package 1.0.0+ only.
    pub fn suit_heart() -> IconData {
        cupertino(0xf833)
    }

    /// Cupertino icon named "suit_heart_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn suit_heart_fill() -> IconData {
        cupertino(0xf834)
    }

    /// Cupertino icon named "suit_spade". Available on cupertino_icons package 1.0.0+ only.
    pub fn suit_spade() -> IconData {
        cupertino(0xf835)
    }

    /// Cupertino icon named "suit_spade_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn suit_spade_fill() -> IconData {
        cupertino(0xf836)
    }

    /// Cupertino icon named "sum". Available on cupertino_icons package 1.0.0+ only.
    pub fn sum() -> IconData {
        cupertino(0xf837)
    }

    /// Cupertino icon named "sun_dust". Available on cupertino_icons package 1.0.0+ only.
    pub fn sun_dust() -> IconData {
        cupertino(0xf838)
    }

    /// Cupertino icon named "sun_dust_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn sun_dust_fill() -> IconData {
        cupertino(0xf839)
    }

    /// Cupertino icon named "sun_haze". Available on cupertino_icons package 1.0.0+ only.
    pub fn sun_haze() -> IconData {
        cupertino(0xf83a)
    }

    /// Cupertino icon named "sun_haze_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn sun_haze_fill() -> IconData {
        cupertino(0xf83b)
    }

    /// Cupertino icon named "sun_max". Available on cupertino_icons package 1.0.0+ only.
    /// This is the same icon as [`brightness`](Self::brightness()) which is available in cupertino_icons 0.1.3.
    pub fn sun_max() -> IconData {
        cupertino(0xf4b6)
    }

    /// Cupertino icon named "sun_max_fill". Available on cupertino_icons package 1.0.0+ only.
    /// This is the same icon as [`brightness_solid`](Self::brightness_solid()) which is available in cupertino_icons 0.1.3.
    pub fn sun_max_fill() -> IconData {
        cupertino(0xf4b7)
    }

    /// Cupertino icon named "sun_min". Available on cupertino_icons package 1.0.0+ only.
    pub fn sun_min() -> IconData {
        cupertino(0xf83c)
    }

    /// Cupertino icon named "sun_min_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn sun_min_fill() -> IconData {
        cupertino(0xf83d)
    }

    /// Cupertino icon named "sunrise". Available on cupertino_icons package 1.0.0+ only.
    pub fn sunrise() -> IconData {
        cupertino(0xf83e)
    }

    /// Cupertino icon named "sunrise_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn sunrise_fill() -> IconData {
        cupertino(0xf83f)
    }

    /// Cupertino icon named "sunset". Available on cupertino_icons package 1.0.0+ only.
    pub fn sunset() -> IconData {
        cupertino(0xf840)
    }

    /// Cupertino icon named "sunset_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn sunset_fill() -> IconData {
        cupertino(0xf841)
    }

    /// Cupertino icon named "t_bubble". Available on cupertino_icons package 1.0.0+ only.
    pub fn t_bubble() -> IconData {
        cupertino(0xf842)
    }

    /// Cupertino icon named "t_bubble_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn t_bubble_fill() -> IconData {
        cupertino(0xf843)
    }

    /// Cupertino icon named "table". Available on cupertino_icons package 1.0.0+ only.
    pub fn table() -> IconData {
        cupertino(0xf844)
    }

    /// Cupertino icon named "table_badge_more". Available on cupertino_icons package 1.0.0+ only.
    pub fn table_badge_more() -> IconData {
        cupertino(0xf845)
    }

    /// Cupertino icon named "table_badge_more_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn table_badge_more_fill() -> IconData {
        cupertino(0xf846)
    }

    /// Cupertino icon named "table_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn table_fill() -> IconData {
        cupertino(0xf847)
    }

    /// Cupertino icon named "tag_circle". Available on cupertino_icons package 1.0.0+ only.
    pub fn tag_circle() -> IconData {
        cupertino(0xf848)
    }

    /// Cupertino icon named "tag_circle_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn tag_circle_fill() -> IconData {
        cupertino(0xf849)
    }

    /// Cupertino icon named "tag_fill". Available on cupertino_icons package 1.0.0+ only.
    /// This is the same icon as [`tag_solid`](Self::tag_solid()) which is available in cupertino_icons 0.1.3.
    /// This is the same icon as [`tags_solid`](Self::tags_solid()) which is available in cupertino_icons 0.1.3.
    pub fn tag_fill() -> IconData {
        cupertino(0xf48d)
    }

    /// Cupertino icon named "text_aligncenter". Available on cupertino_icons package 1.0.0+ only.
    pub fn text_aligncenter() -> IconData {
        cupertino(0xf84a)
    }

    /// Cupertino icon named "text_alignleft". Available on cupertino_icons package 1.0.0+ only.
    pub fn text_alignleft() -> IconData {
        cupertino(0xf84b)
    }

    /// Cupertino icon named "text_alignright". Available on cupertino_icons package 1.0.0+ only.
    pub fn text_alignright() -> IconData {
        cupertino(0xf84c)
    }

    /// Cupertino icon named "text_append". Available on cupertino_icons package 1.0.0+ only.
    pub fn text_append() -> IconData {
        cupertino(0xf84d)
    }

    /// Cupertino icon named "text_badge_checkmark". Available on cupertino_icons package 1.0.0+ only.
    pub fn text_badge_checkmark() -> IconData {
        cupertino(0xf84e)
    }

    /// Cupertino icon named "text_badge_minus". Available on cupertino_icons package 1.0.0+ only.
    pub fn text_badge_minus() -> IconData {
        cupertino(0xf84f)
    }

    /// Cupertino icon named "text_badge_plus". Available on cupertino_icons package 1.0.0+ only.
    pub fn text_badge_plus() -> IconData {
        cupertino(0xf850)
    }

    /// Cupertino icon named "text_badge_star". Available on cupertino_icons package 1.0.0+ only.
    pub fn text_badge_star() -> IconData {
        cupertino(0xf851)
    }

    /// Cupertino icon named "text_badge_xmark". Available on cupertino_icons package 1.0.0+ only.
    pub fn text_badge_xmark() -> IconData {
        cupertino(0xf852)
    }

    /// Cupertino icon named "text_bubble". Available on cupertino_icons package 1.0.0+ only.
    pub fn text_bubble() -> IconData {
        cupertino(0xf853)
    }

    /// Cupertino icon named "text_bubble_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn text_bubble_fill() -> IconData {
        cupertino(0xf854)
    }

    /// Cupertino icon named "text_cursor". Available on cupertino_icons package 1.0.0+ only.
    pub fn text_cursor() -> IconData {
        cupertino(0xf855)
    }

    /// Cupertino icon named "text_insert". Available on cupertino_icons package 1.0.0+ only.
    pub fn text_insert() -> IconData {
        cupertino(0xf856)
    }

    /// Cupertino icon named "text_justify". Available on cupertino_icons package 1.0.0+ only.
    pub fn text_justify() -> IconData {
        cupertino(0xf857)
    }

    /// Cupertino icon named "text_justifyleft". Available on cupertino_icons package 1.0.0+ only.
    pub fn text_justifyleft() -> IconData {
        cupertino(0xf858)
    }

    /// Cupertino icon named "text_justifyright". Available on cupertino_icons package 1.0.0+ only.
    pub fn text_justifyright() -> IconData {
        cupertino(0xf859)
    }

    /// Cupertino icon named "text_quote". Available on cupertino_icons package 1.0.0+ only.
    pub fn text_quote() -> IconData {
        cupertino(0xf85a)
    }

    /// Cupertino icon named "textbox". Available on cupertino_icons package 1.0.0+ only.
    pub fn textbox() -> IconData {
        cupertino(0xf85b)
    }

    /// Cupertino icon named "textformat". Available on cupertino_icons package 1.0.0+ only.
    pub fn textformat() -> IconData {
        cupertino(0xf85c)
    }

    /// Cupertino icon named "textformat_123". Available on cupertino_icons package 1.0.0+ only.
    pub fn textformat_123() -> IconData {
        cupertino(0xf85d)
    }

    /// Cupertino icon named "textformat_abc". Available on cupertino_icons package 1.0.0+ only.
    pub fn textformat_abc() -> IconData {
        cupertino(0xf85e)
    }

    /// Cupertino icon named "textformat_abc_dottedunderline". Available on cupertino_icons package 1.0.0+ only.
    pub fn textformat_abc_dottedunderline() -> IconData {
        cupertino(0xf85f)
    }

    /// Cupertino icon named "textformat_alt". Available on cupertino_icons package 1.0.0+ only.
    pub fn textformat_alt() -> IconData {
        cupertino(0xf860)
    }

    /// Cupertino icon named "textformat_size". Available on cupertino_icons package 1.0.0+ only.
    pub fn textformat_size() -> IconData {
        cupertino(0xf861)
    }

    /// Cupertino icon named "textformat_subscript". Available on cupertino_icons package 1.0.0+ only.
    pub fn textformat_subscript() -> IconData {
        cupertino(0xf862)
    }

    /// Cupertino icon named "textformat_superscript". Available on cupertino_icons package 1.0.0+ only.
    pub fn textformat_superscript() -> IconData {
        cupertino(0xf863)
    }

    /// Cupertino icon named "thermometer". Available on cupertino_icons package 1.0.0+ only.
    pub fn thermometer() -> IconData {
        cupertino(0xf864)
    }

    /// Cupertino icon named "thermometer_snowflake". Available on cupertino_icons package 1.0.0+ only.
    pub fn thermometer_snowflake() -> IconData {
        cupertino(0xf865)
    }

    /// Cupertino icon named "thermometer_sun". Available on cupertino_icons package 1.0.0+ only.
    pub fn thermometer_sun() -> IconData {
        cupertino(0xf866)
    }

    /// Cupertino icon named "ticket". Available on cupertino_icons package 1.0.0+ only.
    pub fn ticket() -> IconData {
        cupertino(0xf916)
    }

    /// Cupertino icon named "ticket_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn ticket_fill() -> IconData {
        cupertino(0xf917)
    }

    /// Cupertino icon named "tickets". Available on cupertino_icons package 1.0.0+ only.
    pub fn tickets() -> IconData {
        cupertino(0xf918)
    }

    /// Cupertino icon named "tickets_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn tickets_fill() -> IconData {
        cupertino(0xf919)
    }

    /// Cupertino icon named "timelapse". Available on cupertino_icons package 1.0.0+ only.
    pub fn timelapse() -> IconData {
        cupertino(0xf867)
    }

    /// Cupertino icon named "timer". Available on cupertino_icons package 1.0.0+ only.
    pub fn timer() -> IconData {
        cupertino(0xf868)
    }

    /// Cupertino icon named "timer_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn timer_fill() -> IconData {
        cupertino(0xf91a)
    }

    /// Cupertino icon named "today". Available on cupertino_icons package 1.0.0+ only.
    pub fn today() -> IconData {
        cupertino(0xf91b)
    }

    /// Cupertino icon named "today_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn today_fill() -> IconData {
        cupertino(0xf91c)
    }

    /// Cupertino icon named "tornado". Available on cupertino_icons package 1.0.0+ only.
    pub fn tornado() -> IconData {
        cupertino(0xf869)
    }

    /// Cupertino icon named "tortoise". Available on cupertino_icons package 1.0.0+ only.
    pub fn tortoise() -> IconData {
        cupertino(0xf86a)
    }

    /// Cupertino icon named "tortoise_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn tortoise_fill() -> IconData {
        cupertino(0xf86b)
    }

    /// Cupertino icon named "tram_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn tram_fill() -> IconData {
        cupertino(0xf86c)
    }

    /// Cupertino icon named "trash". Available on cupertino_icons package 1.0.0+ only.
    /// This is the same icon as [`delete`](Self::delete()) which is available in cupertino_icons 0.1.3.
    /// This is the same icon as [`delete_simple`](Self::delete_simple()) which is available in cupertino_icons 0.1.3.
    pub fn trash() -> IconData {
        cupertino(0xf4c4)
    }

    /// Cupertino icon named "trash_circle". Available on cupertino_icons package 1.0.0+ only.
    pub fn trash_circle() -> IconData {
        cupertino(0xf86d)
    }

    /// Cupertino icon named "trash_circle_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn trash_circle_fill() -> IconData {
        cupertino(0xf86e)
    }

    /// Cupertino icon named "trash_fill". Available on cupertino_icons package 1.0.0+ only.
    /// This is the same icon as [`delete_solid`](Self::delete_solid()) which is available in cupertino_icons 0.1.3.
    pub fn trash_fill() -> IconData {
        cupertino(0xf4c5)
    }

    /// Cupertino icon named "trash_slash". Available on cupertino_icons package 1.0.0+ only.
    pub fn trash_slash() -> IconData {
        cupertino(0xf86f)
    }

    /// Cupertino icon named "trash_slash_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn trash_slash_fill() -> IconData {
        cupertino(0xf870)
    }

    /// Cupertino icon named "tray". Available on cupertino_icons package 1.0.0+ only.
    pub fn tray() -> IconData {
        cupertino(0xf871)
    }

    /// Cupertino icon named "tray_2". Available on cupertino_icons package 1.0.0+ only.
    pub fn tray_2() -> IconData {
        cupertino(0xf872)
    }

    /// Cupertino icon named "tray_2_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn tray_2_fill() -> IconData {
        cupertino(0xf873)
    }

    /// Cupertino icon named "tray_arrow_down". Available on cupertino_icons package 1.0.0+ only.
    pub fn tray_arrow_down() -> IconData {
        cupertino(0xf874)
    }

    /// Cupertino icon named "tray_arrow_down_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn tray_arrow_down_fill() -> IconData {
        cupertino(0xf875)
    }

    /// Cupertino icon named "tray_arrow_up". Available on cupertino_icons package 1.0.0+ only.
    pub fn tray_arrow_up() -> IconData {
        cupertino(0xf876)
    }

    /// Cupertino icon named "tray_arrow_up_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn tray_arrow_up_fill() -> IconData {
        cupertino(0xf877)
    }

    /// Cupertino icon named "tray_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn tray_fill() -> IconData {
        cupertino(0xf878)
    }

    /// Cupertino icon named "tray_full". Available on cupertino_icons package 1.0.0+ only.
    pub fn tray_full() -> IconData {
        cupertino(0xf879)
    }

    /// Cupertino icon named "tray_full_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn tray_full_fill() -> IconData {
        cupertino(0xf87a)
    }

    /// Cupertino icon named "tree". Available on cupertino_icons package 1.0.0+ only.
    pub fn tree() -> IconData {
        cupertino(0xf91d)
    }

    /// Cupertino icon named "triangle". Available on cupertino_icons package 1.0.0+ only.
    pub fn triangle() -> IconData {
        cupertino(0xf87b)
    }

    /// Cupertino icon named "triangle_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn triangle_fill() -> IconData {
        cupertino(0xf87c)
    }

    /// Cupertino icon named "triangle_lefthalf_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn triangle_lefthalf_fill() -> IconData {
        cupertino(0xf87d)
    }

    /// Cupertino icon named "triangle_righthalf_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn triangle_righthalf_fill() -> IconData {
        cupertino(0xf87e)
    }

    /// Cupertino icon named "tropicalstorm". Available on cupertino_icons package 1.0.0+ only.
    pub fn tropicalstorm() -> IconData {
        cupertino(0xf87f)
    }

    /// Cupertino icon named "tuningfork". Available on cupertino_icons package 1.0.0+ only.
    pub fn tuningfork() -> IconData {
        cupertino(0xf880)
    }

    /// Cupertino icon named "tv". Available on cupertino_icons package 1.0.0+ only.
    pub fn tv() -> IconData {
        cupertino(0xf881)
    }

    /// Cupertino icon named "tv_circle". Available on cupertino_icons package 1.0.0+ only.
    pub fn tv_circle() -> IconData {
        cupertino(0xf882)
    }

    /// Cupertino icon named "tv_circle_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn tv_circle_fill() -> IconData {
        cupertino(0xf883)
    }

    /// Cupertino icon named "tv_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn tv_fill() -> IconData {
        cupertino(0xf884)
    }

    /// Cupertino icon named "tv_music_note". Available on cupertino_icons package 1.0.0+ only.
    pub fn tv_music_note() -> IconData {
        cupertino(0xf885)
    }

    /// Cupertino icon named "tv_music_note_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn tv_music_note_fill() -> IconData {
        cupertino(0xf886)
    }

    /// Cupertino icon named "uiwindow_split_2x1". Available on cupertino_icons package 1.0.0+ only.
    pub fn uiwindow_split_2x1() -> IconData {
        cupertino(0xf887)
    }

    /// Cupertino icon named "umbrella". Available on cupertino_icons package 1.0.0+ only.
    pub fn umbrella() -> IconData {
        cupertino(0xf888)
    }

    /// Cupertino icon named "umbrella_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn umbrella_fill() -> IconData {
        cupertino(0xf889)
    }

    /// Cupertino icon named "underline". Available on cupertino_icons package 1.0.0+ only.
    pub fn underline() -> IconData {
        cupertino(0xf88a)
    }

    /// Cupertino icon named "upload_circle". Available on cupertino_icons package 1.0.0+ only.
    pub fn upload_circle() -> IconData {
        cupertino(0xf91e)
    }

    /// Cupertino icon named "upload_circle_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn upload_circle_fill() -> IconData {
        cupertino(0xf91f)
    }

    /// Cupertino icon named "videocam". Available on cupertino_icons package 1.0.0+ only.
    /// This is the same icon as [`video_camera`](Self::video_camera()) which is available in cupertino_icons 0.1.3.
    pub fn videocam() -> IconData {
        cupertino(0xf4cc)
    }

    /// Cupertino icon named "videocam_circle". Available on cupertino_icons package 1.0.0+ only.
    pub fn videocam_circle() -> IconData {
        cupertino(0xf920)
    }

    /// Cupertino icon named "videocam_circle_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn videocam_circle_fill() -> IconData {
        cupertino(0xf921)
    }

    /// Cupertino icon named "videocam_fill". Available on cupertino_icons package 1.0.0+ only.
    /// This is the same icon as [`video_camera_solid`](Self::video_camera_solid()) which is available in cupertino_icons 0.1.3.
    pub fn videocam_fill() -> IconData {
        cupertino(0xf4cd)
    }

    /// Cupertino icon named "view_2d". Available on cupertino_icons package 1.0.0+ only.
    pub fn view_2d() -> IconData {
        cupertino(0xf88b)
    }

    /// Cupertino icon named "view_3d". Available on cupertino_icons package 1.0.0+ only.
    pub fn view_3d() -> IconData {
        cupertino(0xf88c)
    }

    /// Cupertino icon named "viewfinder". Available on cupertino_icons package 1.0.0+ only.
    pub fn viewfinder() -> IconData {
        cupertino(0xf88d)
    }

    /// Cupertino icon named "viewfinder_circle". Available on cupertino_icons package 1.0.0+ only.
    pub fn viewfinder_circle() -> IconData {
        cupertino(0xf88e)
    }

    /// Cupertino icon named "viewfinder_circle_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn viewfinder_circle_fill() -> IconData {
        cupertino(0xf88f)
    }

    /// Cupertino icon named "wand_rays". Available on cupertino_icons package 1.0.0+ only.
    pub fn wand_rays() -> IconData {
        cupertino(0xf890)
    }

    /// Cupertino icon named "wand_rays_inverse". Available on cupertino_icons package 1.0.0+ only.
    pub fn wand_rays_inverse() -> IconData {
        cupertino(0xf891)
    }

    /// Cupertino icon named "wand_stars". Available on cupertino_icons package 1.0.0+ only.
    pub fn wand_stars() -> IconData {
        cupertino(0xf892)
    }

    /// Cupertino icon named "wand_stars_inverse". Available on cupertino_icons package 1.0.0+ only.
    pub fn wand_stars_inverse() -> IconData {
        cupertino(0xf893)
    }

    /// Cupertino icon named "waveform". Available on cupertino_icons package 1.0.0+ only.
    pub fn waveform() -> IconData {
        cupertino(0xf894)
    }

    /// Cupertino icon named "waveform_circle". Available on cupertino_icons package 1.0.0+ only.
    pub fn waveform_circle() -> IconData {
        cupertino(0xf895)
    }

    /// Cupertino icon named "waveform_circle_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn waveform_circle_fill() -> IconData {
        cupertino(0xf896)
    }

    /// Cupertino icon named "waveform_path". Available on cupertino_icons package 1.0.0+ only.
    pub fn waveform_path() -> IconData {
        cupertino(0xf897)
    }

    /// Cupertino icon named "waveform_path_badge_minus". Available on cupertino_icons package 1.0.0+ only.
    pub fn waveform_path_badge_minus() -> IconData {
        cupertino(0xf898)
    }

    /// Cupertino icon named "waveform_path_badge_plus". Available on cupertino_icons package 1.0.0+ only.
    pub fn waveform_path_badge_plus() -> IconData {
        cupertino(0xf899)
    }

    /// Cupertino icon named "waveform_path_ecg". Available on cupertino_icons package 1.0.0+ only.
    pub fn waveform_path_ecg() -> IconData {
        cupertino(0xf89a)
    }

    /// Cupertino icon named "wifi". Available on cupertino_icons package 1.0.0+ only.
    pub fn wifi() -> IconData {
        cupertino(0xf89b)
    }

    /// Cupertino icon named "wifi_exclamationmark". Available on cupertino_icons package 1.0.0+ only.
    pub fn wifi_exclamationmark() -> IconData {
        cupertino(0xf89c)
    }

    /// Cupertino icon named "wifi_slash". Available on cupertino_icons package 1.0.0+ only.
    pub fn wifi_slash() -> IconData {
        cupertino(0xf89d)
    }

    /// Cupertino icon named "wind". Available on cupertino_icons package 1.0.0+ only.
    pub fn wind() -> IconData {
        cupertino(0xf89e)
    }

    /// Cupertino icon named "wind_snow". Available on cupertino_icons package 1.0.0+ only.
    pub fn wind_snow() -> IconData {
        cupertino(0xf89f)
    }

    /// Cupertino icon named "wrench". Available on cupertino_icons package 1.0.0+ only.
    pub fn wrench() -> IconData {
        cupertino(0xf8a0)
    }

    /// Cupertino icon named "wrench_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn wrench_fill() -> IconData {
        cupertino(0xf8a1)
    }

    /// Cupertino icon named "xmark". Available on cupertino_icons package 1.0.0+ only.
    /// This is the same icon as [`clear_thick`](Self::clear_thick()) which is available in cupertino_icons 0.1.3.
    /// This is the same icon as [`clear`](Self::clear()) which is available in cupertino_icons 0.1.3.
    pub fn xmark() -> IconData {
        cupertino(0xf404)
    }

    /// Cupertino icon named "xmark_circle". Available on cupertino_icons package 1.0.0+ only.
    /// This is the same icon as [`clear_circled`](Self::clear_circled()) which is available in cupertino_icons 0.1.3.
    pub fn xmark_circle() -> IconData {
        cupertino(0xf405)
    }

    /// Cupertino icon named "xmark_circle_fill". Available on cupertino_icons package 1.0.0+ only.
    /// This is the same icon as [`clear_thick_circled`](Self::clear_thick_circled()) which is available in cupertino_icons 0.1.3.
    /// This is the same icon as [`clear_circled_solid`](Self::clear_circled_solid()) which is available in cupertino_icons 0.1.3.
    pub fn xmark_circle_fill() -> IconData {
        cupertino(0xf36e)
    }

    /// Cupertino icon named "xmark_octagon". Available on cupertino_icons package 1.0.0+ only.
    pub fn xmark_octagon() -> IconData {
        cupertino(0xf8a2)
    }

    /// Cupertino icon named "xmark_octagon_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn xmark_octagon_fill() -> IconData {
        cupertino(0xf8a3)
    }

    /// Cupertino icon named "xmark_rectangle". Available on cupertino_icons package 1.0.0+ only.
    pub fn xmark_rectangle() -> IconData {
        cupertino(0xf8a4)
    }

    /// Cupertino icon named "xmark_rectangle_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn xmark_rectangle_fill() -> IconData {
        cupertino(0xf8a5)
    }

    /// Cupertino icon named "xmark_seal". Available on cupertino_icons package 1.0.0+ only.
    pub fn xmark_seal() -> IconData {
        cupertino(0xf8a6)
    }

    /// Cupertino icon named "xmark_seal_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn xmark_seal_fill() -> IconData {
        cupertino(0xf8a7)
    }

    /// Cupertino icon named "xmark_shield". Available on cupertino_icons package 1.0.0+ only.
    pub fn xmark_shield() -> IconData {
        cupertino(0xf8a8)
    }

    /// Cupertino icon named "xmark_shield_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn xmark_shield_fill() -> IconData {
        cupertino(0xf8a9)
    }

    /// Cupertino icon named "xmark_square". Available on cupertino_icons package 1.0.0+ only.
    pub fn xmark_square() -> IconData {
        cupertino(0xf8aa)
    }

    /// Cupertino icon named "xmark_square_fill". Available on cupertino_icons package 1.0.0+ only.
    pub fn xmark_square_fill() -> IconData {
        cupertino(0xf8ab)
    }

    /// Cupertino icon named "zoom_in". Available on cupertino_icons package 1.0.0+ only.
    pub fn zoom_in() -> IconData {
        cupertino(0xf8ac)
    }

    /// Cupertino icon named "zoom_out". Available on cupertino_icons package 1.0.0+ only.
    pub fn zoom_out() -> IconData {
        cupertino(0xf8ad)
    }

    /// Cupertino icon named "zzz". Available on cupertino_icons package 1.0.0+ only.
    pub fn zzz() -> IconData {
        cupertino(0xf8ae)
    }

    // END GENERATED SF SYMBOLS NAMES
    // ===========================================================================
}

#[cfg(test)]
mod tests {
    use reveal_embedder::{FontCollection, TextDirection};
    use reveal_painting::{TextPainter, TextSpan, TextStyle};

    use super::*;

    /// The glyph of an icon, as the text a paragraph shapes.
    fn glyph(icon: &IconData) -> String {
        char::from_u32(icon.code_point)
            .expect("an icon code point")
            .into()
    }

    /// What an `Icon` widget would style its glyph with.
    fn icon_style() -> TextStyle {
        TextStyle::new()
            .font_size(24.0)
            .font_family(CupertinoIcons::ICON_FONT)
            .package(CupertinoIcons::ICON_FONT_PACKAGE)
    }

    fn installed_fonts(app: &mut App) -> Handle<FontCollection> {
        install_cupertino_icon_font(app);
        PaintingBinding::instance(app).fonts(app)
    }

    #[test]
    fn the_table_carries_the_code_point_the_font_and_the_package() {
        assert_eq!(CupertinoIcons::ICON_FONT, "CupertinoIcons");
        assert_eq!(CupertinoIcons::ICON_FONT_PACKAGE, "cupertino_icons");
        for (icon, code_point) in [
            (CupertinoIcons::left_chevron(), 0xf3d2),
            (CupertinoIcons::share(), 0xf4ca),
            (CupertinoIcons::heart_fill(), 0xf443),
            (CupertinoIcons::bell_fill(), 0xf3e2),
            (CupertinoIcons::umbrella_fill(), 0xf889),
            (CupertinoIcons::r#loop(), 0xf449),
            (CupertinoIcons::r#move(), 0xf8f8),
            (CupertinoIcons::zzz(), 0xf8ae),
        ] {
            assert_eq!(icon.code_point, code_point);
            assert_eq!(icon.font_family.as_deref(), Some("CupertinoIcons"));
            assert_eq!(icon.font_package.as_deref(), Some("cupertino_icons"));
        }
    }

    #[test]
    fn only_the_navigation_chevrons_mirror_in_rtl() {
        assert!(CupertinoIcons::left_chevron().match_text_direction);
        assert!(CupertinoIcons::right_chevron().match_text_direction);
        assert!(CupertinoIcons::back().match_text_direction);
        assert!(CupertinoIcons::forward().match_text_direction);
        assert!(!CupertinoIcons::heart().match_text_direction);
    }

    #[test]
    fn the_style_an_icon_asks_for_names_the_family_the_font_is_registered_under() {
        assert_eq!(
            icon_style().font_family.as_deref(),
            Some(icon_font_family().as_str())
        );
    }

    #[test]
    fn installing_the_font_registers_a_family_covering_the_icon_glyphs() {
        let mut app = App::new();
        let fonts = installed_fonts(&mut app);
        let collection = app.get(fonts);
        let id = collection
            .family(&icon_font_family())
            .expect("the icon family");
        for icon in [CupertinoIcons::heart(), CupertinoIcons::zzz()] {
            let glyph = char::from_u32(icon.code_point).expect("an icon code point");
            assert!(collection.get(id).covers(glyph));
        }
    }

    #[test]
    fn installing_the_font_twice_registers_it_once() {
        let mut app = App::new();
        let fonts = installed_fonts(&mut app);
        install_cupertino_icon_font(&mut app);
        assert_eq!(app.get(fonts).len(), 1);
    }

    #[test]
    fn an_installed_icon_lays_out_as_one_glyph() {
        let mut app = App::new();
        let fonts = installed_fonts(&mut app);
        let mut painter = TextPainter::new();
        painter.set_text(Some(
            TextSpan::new()
                .text(glyph(&CupertinoIcons::heart_fill()))
                .style(icon_style())
                .into_span(),
        ));
        painter.set_text_direction(Some(TextDirection::Ltr));
        painter.layout(app.get_mut(fonts), 0.0, f64::INFINITY);
        assert!(painter.width() > 0.0 && painter.height() > 0.0);
        assert!(
            app.get_mut(fonts).take_unanswered().is_empty(),
            "the icon family and its glyph are covered by the bundled font"
        );
        painter.dispose();
    }
}
