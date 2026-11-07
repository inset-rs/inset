//! The user's caption settings on Apple's systems: Media Accessibility's caption appearance
//! (`MACaptionAppearance`), its style read from the user's domain at each ask, as Chromium's
//! `caption_style_mac.mm` and WebKit's `CaptionUserPreferencesMediaAF` read it, and its change
//! notice (`kMACaptionAppearanceSettingsChangedNotification`), which WebKit hears on the process's
//! own notification center, resolving the settings' change futures.
//!
//! A setting applies always where Apple's behaviour is `UseValue` and the user customized the
//! active style, as WebKit marks such a setting `!important` (`behaviorShouldNotBeOverriden`); every
//! other setting applies where the media says nothing. Colours come opaque, their opacities read
//! on their own, as each has a behaviour of its own.

// Media Accessibility's functions and Core Graphics' colour matching need `unsafe`; this module
// writes it for them.
#![allow(unsafe_code)]

use std::ptr::NonNull;
use std::rc::Rc;

use block2::RcBlock;
use objc2::rc::Retained;
use objc2::runtime::{NSObjectProtocol, ProtocolObject};
use objc2_core_foundation::CFString;
use objc2_core_graphics::{CGColor, CGColorRenderingIntent, CGColorSpace, kCGColorSpaceSRGB};
use objc2_core_text::kCTFontFamilyNameAttribute;
use objc2_foundation::{NSNotification, NSNotificationCenter, NSOperationQueue};
use objc2_media_accessibility::{
    MACaptionAppearanceBehavior, MACaptionAppearanceCopyBackgroundColor,
    MACaptionAppearanceCopyFontDescriptorForStyle, MACaptionAppearanceCopyForegroundColor,
    MACaptionAppearanceCopyWindowColor, MACaptionAppearanceDomain, MACaptionAppearanceFontStyle,
    MACaptionAppearanceGetBackgroundOpacity, MACaptionAppearanceGetForegroundOpacity,
    MACaptionAppearanceGetRelativeCharacterSize, MACaptionAppearanceGetTextEdgeStyle,
    MACaptionAppearanceGetWindowOpacity, MACaptionAppearanceGetWindowRoundedCornerRadius,
    MACaptionAppearanceIsCustomized, MACaptionAppearanceTextEdgeStyle,
    kMACaptionAppearanceSettingsChangedNotification,
};
use waft_embedder::{
    CaptionBehavior, CaptionSetting, CaptionSettings, CaptionStyle, CaptionTypeface, Color,
    ColorSpace, CueTextEdge, MediaChangeFuture,
};

use super::media::{abort_on_panic, as_ns_string};
use crate::media::Changes;

const USER: MACaptionAppearanceDomain = MACaptionAppearanceDomain::User;

/// Media Accessibility's caption settings.
pub(crate) fn caption_settings() -> Option<Rc<dyn CaptionSettings>> {
    Some(Rc::new(CaptionAppearance::new()))
}

/// The user's caption settings, whose change notice is heard on the main queue until they are
/// dropped.
struct CaptionAppearance {
    changes: Changes,
    observation: Retained<ProtocolObject<dyn NSObjectProtocol>>,
}

impl CaptionAppearance {
    fn new() -> CaptionAppearance {
        let changes = Changes::default();
        let observation = hear_changes({
            let changes = changes.clone();
            move || changes.notify()
        });
        CaptionAppearance {
            changes,
            observation,
        }
    }
}

impl CaptionSettings for CaptionAppearance {
    fn style(&self) -> CaptionStyle {
        caption_style()
    }

    fn changed(&self) -> MediaChangeFuture {
        self.changes.next()
    }
}

impl Drop for CaptionAppearance {
    fn drop(&mut self) {
        // SAFETY: a token `addObserverForName:object:queue:usingBlock:` answered.
        unsafe { NSNotificationCenter::defaultCenter().removeObserver(self.observation.as_ref()) };
    }
}

/// The user's caption style now.
fn caption_style() -> CaptionStyle {
    // SAFETY: a plain query of the user's domain.
    let customized = unsafe { MACaptionAppearanceIsCustomized(USER) };
    let read = Settings { customized };
    // SAFETY (each read below): a query of the user's domain, writing the setting's behaviour
    // where it is pointed.
    CaptionStyle {
        foreground_color: Some(read.setting(|behavior| unsafe {
            opaque(&MACaptionAppearanceCopyForegroundColor(USER, behavior))
        })),
        foreground_opacity: Some(read.setting(|behavior| unsafe {
            MACaptionAppearanceGetForegroundOpacity(USER, behavior)
        })),
        background_color: Some(read.setting(|behavior| unsafe {
            opaque(&MACaptionAppearanceCopyBackgroundColor(USER, behavior))
        })),
        background_opacity: Some(read.setting(|behavior| unsafe {
            MACaptionAppearanceGetBackgroundOpacity(USER, behavior)
        })),
        window_color: Some(read.setting(|behavior| unsafe {
            opaque(&MACaptionAppearanceCopyWindowColor(USER, behavior))
        })),
        window_opacity: Some(
            read.setting(|behavior| unsafe { MACaptionAppearanceGetWindowOpacity(USER, behavior) }),
        ),
        window_corner_radius: Some(read.setting(|behavior| unsafe {
            MACaptionAppearanceGetWindowRoundedCornerRadius(USER, behavior)
        })),
        edge: said(
            read.setting(|behavior| unsafe { MACaptionAppearanceGetTextEdgeStyle(USER, behavior) }),
            edge_of,
        ),
        edge_color: None,
        typeface: said(
            read.setting(|behavior| unsafe {
                MACaptionAppearanceCopyFontDescriptorForStyle(
                    USER,
                    behavior,
                    MACaptionAppearanceFontStyle::Default,
                )
            }),
            |font| family_of(&font).map(CaptionTypeface::Family),
        ),
        // Zero where the user set none, as WebKit reads it.
        text_scale: said(
            read.setting(|behavior| unsafe {
                MACaptionAppearanceGetRelativeCharacterSize(USER, behavior)
            }),
            |scale| (scale > 0.0).then_some(scale),
        ),
    }
}

/// Calls `heard` on the main thread each time the user changes a caption setting, until the
/// observation answered is removed from the default center.
fn hear_changes(heard: impl Fn() + 'static) -> Retained<ProtocolObject<dyn NSObjectProtocol>> {
    let block = RcBlock::new(move |_notification: NonNull<NSNotification>| abort_on_panic(&heard));
    // SAFETY: a constant Media Accessibility exports for the life of the process, an NSString by
    // toll-free bridging.
    let name = as_ns_string(unsafe { kMACaptionAppearanceSettingsChangedNotification });
    // SAFETY: the block lives as long as the observation, which its owner removes, and the main
    // queue serves it on the main thread.
    unsafe {
        NSNotificationCenter::defaultCenter().addObserverForName_object_queue_usingBlock(
            Some(name),
            None,
            Some(&NSOperationQueue::mainQueue()),
            &block,
        )
    }
}

/// Reads settings of a style, customized or not.
struct Settings {
    customized: bool,
}

impl Settings {
    /// The value `read` answers, applying as the behaviour it writes says.
    fn setting<T>(
        &self,
        read: impl FnOnce(*mut MACaptionAppearanceBehavior) -> T,
    ) -> CaptionSetting<T> {
        let mut behavior = MACaptionAppearanceBehavior::UseContentIfAvailable;
        let value = read(&mut behavior);
        CaptionSetting {
            value,
            behavior: behavior_of(behavior, self.customized),
        }
    }
}

/// `setting` with its value made the framework's, or none where that says nothing.
fn said<T, U>(
    setting: CaptionSetting<T>,
    value_of: impl FnOnce(T) -> Option<U>,
) -> Option<CaptionSetting<U>> {
    Some(CaptionSetting {
        value: value_of(setting.value)?,
        behavior: setting.behavior,
    })
}

/// When a setting applies: always for Apple's `UseValue` in a style the user customized, as
/// WebKit's `behaviorShouldNotBeOverriden` has it, and where the media says nothing otherwise.
fn behavior_of(behavior: MACaptionAppearanceBehavior, customized: bool) -> CaptionBehavior {
    if behavior == MACaptionAppearanceBehavior::UseValue && customized {
        CaptionBehavior::Always
    } else {
        CaptionBehavior::WhereMediaSaysNothing
    }
}

/// Apple's edge style; none for `Undefined`, which Chromium and WebKit read as no setting.
fn edge_of(style: MACaptionAppearanceTextEdgeStyle) -> Option<CueTextEdge> {
    match style {
        MACaptionAppearanceTextEdgeStyle::None => Some(CueTextEdge::None),
        MACaptionAppearanceTextEdgeStyle::Raised => Some(CueTextEdge::Raised),
        MACaptionAppearanceTextEdgeStyle::Depressed => Some(CueTextEdge::Depressed),
        MACaptionAppearanceTextEdgeStyle::Uniform => Some(CueTextEdge::Outline),
        MACaptionAppearanceTextEdgeStyle::DropShadow => Some(CueTextEdge::DropShadow),
        _ => None,
    }
}

/// The family of `font`; none for one of the system's hidden fonts, whose names start with a dot
/// and which WebKit draws as the system's font, as the framework draws a run of no family.
fn family_of(font: &objc2_core_text::CTFontDescriptor) -> Option<String> {
    // SAFETY: a constant Core Text exports for the life of the process, as the attribute asked.
    let family = unsafe { font.attribute(kCTFontFamilyNameAttribute) }?;
    let family = family.downcast_ref::<CFString>()?.to_string();
    (!family.starts_with('.')).then_some(family)
}

/// `color` in sRGB, opaque, its opacity being a setting of its own; white where Core Graphics
/// cannot match it to sRGB.
fn opaque(color: &CGColor) -> Color {
    let srgb = CGColorSpace::with_name(Some(
        // SAFETY: a constant Core Graphics exports for the life of the process.
        unsafe { kCGColorSpaceSRGB },
    ));
    // SAFETY: no options.
    let matched = unsafe {
        CGColor::new_copy_by_matching_to_color_space(
            srgb.as_deref(),
            CGColorRenderingIntent::RenderingIntentDefault,
            Some(color),
            None,
        )
    };
    let Some(matched) = matched else {
        return Color::new(0xFFFF_FFFF);
    };
    let count = CGColor::number_of_components(Some(&matched));
    let components = CGColor::components(Some(&matched));
    if count < 3 || components.is_null() {
        return Color::new(0xFFFF_FFFF);
    }
    // SAFETY: Core Graphics answers `count` components for a colour it keeps alive.
    let components = unsafe { std::slice::from_raw_parts(components, count) };
    Color::from(
        1.0,
        components[0].clamp(0.0, 1.0),
        components[1].clamp(0.0, 1.0),
        components[2].clamp(0.0, 1.0),
        ColorSpace::Srgb,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_setting_applies_always_only_where_apple_says_use_it_in_a_customized_style() {
        let use_value = MACaptionAppearanceBehavior::UseValue;
        let use_content = MACaptionAppearanceBehavior::UseContentIfAvailable;
        assert_eq!(behavior_of(use_value, true), CaptionBehavior::Always);
        assert_eq!(
            behavior_of(use_value, false),
            CaptionBehavior::WhereMediaSaysNothing,
            "the system's own style yields to the media, as WebKit has it"
        );
        assert_eq!(
            behavior_of(use_content, true),
            CaptionBehavior::WhereMediaSaysNothing
        );
    }

    #[test]
    fn apples_edge_styles_are_the_cues_and_undefined_says_nothing() {
        assert_eq!(
            edge_of(MACaptionAppearanceTextEdgeStyle::Uniform),
            Some(CueTextEdge::Outline)
        );
        assert_eq!(
            edge_of(MACaptionAppearanceTextEdgeStyle::None),
            Some(CueTextEdge::None)
        );
        assert_eq!(edge_of(MACaptionAppearanceTextEdgeStyle::Undefined), None);
    }

    #[test]
    fn a_colour_is_read_in_srgb_without_its_alpha() {
        let color = CGColor::new_srgb(1.0, 0.5, 0.0, 0.25);
        assert_eq!(
            opaque(&color),
            Color::from(1.0, 1.0, 0.5, 0.0, ColorSpace::Srgb)
        );
    }

    #[test]
    fn the_users_style_reads_every_setting_apple_has() {
        let style = caption_style();
        assert!(style.foreground_color.is_some() && style.foreground_opacity.is_some());
        assert!(style.background_color.is_some() && style.window_color.is_some());
        assert_eq!(style.edge_color, None, "Apple gives no edge colour");
    }
}
