//! `CupertinoSwitch`: a tap or a drag, the colours, the on/off labels, a thumb icon.
//!
//! The switch stores no value. `on_changed` reports the flip and nothing moves until this page
//! rebuilds it — which is why a settings row whose tile is tapped flips the same switch. Drag
//! the thumb: it commits past 70% of the track, and dragging back past 20% undoes it before the
//! finger lifts. On a desktop, Tab reaches each switch and draws its focus ring.

use std::rc::Rc;

use waft_cupertino::{
    CupertinoColors, CupertinoIcons, CupertinoListTile, CupertinoSwitch, CupertinoTheme,
};
use waft_embedder::Color;
use waft_foundation::{App, Handle, Task, ValueChanged};
use waft_widgets::{
    BuildContext, Icon, IntoWidget, MediaQuery, State, StateData, StatefulWidget, Text, WidgetRef,
    WidgetState, WidgetStateProperty,
};

use crate::support::{screen, scrolling_body, secondary, section_with_footer, showcase};

pub fn body(_app: &mut App, _context: BuildContext) -> WidgetRef {
    SwitchesDemo.into_widget()
}

/// Every value this page shows lives here, in the smallest widget that renders them.
#[derive(Debug)]
pub struct SwitchesDemo;

/// The settings the page's switches stand for.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Setting {
    AirplaneMode,
    WiFi,
    Tinted,
    Labelled,
    Icons,
    Themed,
}

/// Dart would spell this `_SwitchesDemoState`.
pub struct Settings {
    state: StateData<SwitchesDemo>,
    airplane_mode: bool,
    wi_fi: bool,
    tinted: bool,
    labelled: bool,
    icons: bool,
    themed: bool,
}

impl StatefulWidget for SwitchesDemo {
    type State = Settings;

    fn create_state(&self) -> Settings {
        Settings {
            state: StateData::new(),
            airplane_mode: false,
            wi_fi: true,
            tinted: true,
            labelled: false,
            icons: true,
            themed: true,
        }
    }
}

impl Settings {
    fn value(&self, setting: Setting) -> bool {
        match setting {
            Setting::AirplaneMode => self.airplane_mode,
            Setting::WiFi => self.wi_fi,
            Setting::Tinted => self.tinted,
            Setting::Labelled => self.labelled,
            Setting::Icons => self.icons,
            Setting::Themed => self.themed,
        }
    }

    fn slot(&mut self, setting: Setting) -> &mut bool {
        match setting {
            Setting::AirplaneMode => &mut self.airplane_mode,
            Setting::WiFi => &mut self.wi_fi,
            Setting::Tinted => &mut self.tinted,
            Setting::Labelled => &mut self.labelled,
            Setting::Icons => &mut self.icons,
            Setting::Themed => &mut self.themed,
        }
    }

    /// The callback a switch reports to: it writes the setting and rebuilds the page.
    fn on_changed(this: Handle<Settings>, setting: Setting) -> ValueChanged<bool> {
        Rc::new(move |app: &mut App, value| {
            this.set_state(app, |state| *state.slot(setting) = value);
        })
    }

    /// A switch bound to one setting.
    fn switch(this: Handle<Settings>, app: &App, setting: Setting) -> CupertinoSwitch {
        CupertinoSwitch::new(
            app.get(this).value(setting),
            Some(Settings::on_changed(this, setting)),
        )
    }
}

impl State for Settings {
    type Widget = SwitchesDemo;
    waft_widgets::state_accessors!();

    fn build(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        let summary = format!(
            "Airplane Mode {}, Wi-Fi {}",
            on_off(app.get(self).airplane_mode),
            on_off(app.get(self).wi_fi)
        );
        screen(scrolling_body(vec![
            section_with_footer(
                "Settings",
                "The tile's tap and the switch's own report to the same field.",
                vec![
                    setting_row(self, app, "Airplane Mode", Setting::AirplaneMode),
                    setting_row(self, app, "Wi-Fi", Setting::WiFi),
                    showcase(secondary(app, context, summary)),
                ],
            ),
            section_with_footer(
                "Colours",
                "active_track_color, inactive_track_color, and the two thumb colours.",
                vec![showcase(
                    Settings::switch(self, app, Setting::Tinted)
                        .active_track_color(CupertinoColors::SYSTEM_INDIGO)
                        .inactive_track_color(CupertinoColors::SYSTEM_ORANGE)
                        .thumb_color(CupertinoColors::WHITE)
                        .inactive_thumb_color(Color::from_argb(255, 255, 236, 204)),
                )],
            ),
            section_with_footer(
                "On/off labels",
                "The | and O iOS draws when the accessibility setting asks for them.",
                vec![showcase(labelled(
                    Settings::switch(self, app, Setting::Labelled),
                    app,
                    context,
                ))],
            ),
            section_with_footer(
                "Thumb icon",
                "thumb_icon resolves in the switch's states: a check on, a cross off.",
                vec![showcase(
                    Settings::switch(self, app, Setting::Icons).thumb_icon(thumb_icons()),
                )],
            ),
            section_with_footer(
                "Theme",
                "apply_theme takes the theme's primary colour for the on track.",
                vec![showcase(CupertinoTheme::new(
                    CupertinoTheme::of(app, context)
                        .copy_with()
                        .with_primary_color(CupertinoColors::SYSTEM_PINK),
                    Settings::switch(self, app, Setting::Themed).apply_theme(true),
                ))],
            ),
            section_with_footer(
                "Disabled",
                "With no on_changed the switch is at half opacity and takes no input.",
                vec![
                    showcase(CupertinoSwitch::new(true, None)),
                    showcase(CupertinoSwitch::new(false, None)),
                ],
            ),
        ]))
    }
}

fn on_off(value: bool) -> &'static str {
    if value { "on" } else { "off" }
}

/// A settings row: the tile toggles the value as the switch in it does.
fn setting_row(this: Handle<Settings>, app: &App, title: &str, setting: Setting) -> WidgetRef {
    let on_changed = Settings::on_changed(this, setting);
    CupertinoListTile::notched(Text::new(title))
        .trailing(Settings::switch(this, app, setting))
        .on_tap(Rc::new(move |app: &mut App| {
            let value = app.get(this).value(setting);
            on_changed(app, !value);
            Task::ready(())
        }))
        .into_widget()
}

/// The switch under a media query that asks for on/off labels, as iOS's "On/Off Labels"
/// accessibility setting does.
fn labelled(switch: CupertinoSwitch, app: &mut App, context: BuildContext) -> WidgetRef {
    let data = MediaQuery::of(app, context)
        .copy_with()
        .on_off_switch_labels(true);
    MediaQuery::new(data, switch).into_widget()
}

fn thumb_icons() -> Rc<dyn WidgetStateProperty<Option<Rc<Icon>>>> {
    let check =
        Rc::new(Icon::new(Some(CupertinoIcons::checkmark())).color(CupertinoColors::SYSTEM_GREEN));
    let cross =
        Rc::new(Icon::new(Some(CupertinoIcons::xmark())).color(CupertinoColors::SYSTEM_GREY));
    <dyn WidgetStateProperty<Option<Rc<Icon>>>>::resolve_with(move |states| {
        Some(if states.contains(&WidgetState::Selected) {
            Rc::clone(&check)
        } else {
            Rc::clone(&cross)
        })
    })
}
