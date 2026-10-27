//! ToggleSplitButton.cpp: toggle before primary activation, with the shared SplitButton template.

use super::split_button::SplitButtonPresenter;
use crate::{Flyout, SplitButton};
use inset_foundation::{App, Handle, Listener};
use inset_widgets::*;
use std::{fmt, rc::Rc};

/// Reports the requested checked value before the primary Click callback.
pub type ToggleSplitButtonCheckedHandler = Rc<dyn Fn(&mut App, bool)>;

/// A toggle action with a separate flyout button that does not change the checked value.
#[derive(Clone)]
pub struct ToggleSplitButton {
    /// Current application-owned checked value.
    pub is_checked: bool,

    /// Applies the checked value requested by primary activation.
    pub is_checked_changed: ToggleSplitButtonCheckedHandler,

    /// Content displayed by the primary button.
    pub content: WidgetRef,

    /// Optional primary Click event, raised after the checked-value callback.
    pub click: Option<Listener>,

    /// Flyout opened by the secondary button, F4 or Alt+Down.
    pub flyout: Option<Handle<Flyout>>,

    /// Enables both actions and keyboard focus.
    pub is_enabled: bool,

    /// Includes this control in sequential keyboard traversal.
    pub is_tab_stop: bool,

    /// Optional caller-owned focus identity for the whole control.
    pub focus_node: Option<AnyFocusNode>,

    /// Identity across parent rebuilds.
    pub key: Option<KeyRef>,
}

impl fmt::Debug for ToggleSplitButton {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ToggleSplitButton")
            .field("is_checked", &self.is_checked)
            .field("is_enabled", &self.is_enabled)
            .field("flyout", &self.flyout)
            .finish_non_exhaustive()
    }
}

impl ToggleSplitButton {
    /// Creates a toggle split button using the kit's owner-managed checked-value convention.
    pub fn new<K>(
        content: impl IntoWidget<K>,
        is_checked: bool,
        is_checked_changed: impl Fn(&mut App, bool) + 'static,
    ) -> Self {
        Self {
            content: content.into_widget(),
            is_checked,
            is_checked_changed: Rc::new(is_checked_changed),
            click: None,
            flyout: None,
            is_enabled: true,
            is_tab_stop: true,
            focus_node: None,
            key: None,
        }
    }

    /// Creates a toggle split button with a text label.
    pub fn text(
        text: impl Into<String>,
        is_checked: bool,
        is_checked_changed: impl Fn(&mut App, bool) + 'static,
    ) -> Self {
        Self::new(Text::new(text), is_checked, is_checked_changed)
    }

    /// Sets the Click callback that follows the checked-value callback.
    pub fn click(mut self, click: Listener) -> Self {
        self.click = Some(click);
        self
    }

    /// Sets the flyout without changing its ownership.
    pub fn flyout(mut self, flyout: Handle<Flyout>) -> Self {
        self.flyout = Some(flyout);
        self
    }

    /// Enables or disables both actions.
    pub fn is_enabled(mut self, value: bool) -> Self {
        self.is_enabled = value;
        self
    }

    /// Sets whether Tab visits the control.
    pub fn is_tab_stop(mut self, value: bool) -> Self {
        self.is_tab_stop = value;
        self
    }

    /// Supplies a caller-owned focus node.
    pub fn focus_node(mut self, value: AnyFocusNode) -> Self {
        self.focus_node = Some(value);
        self
    }

    /// Sets the widget's identity.
    pub fn key(mut self, value: KeyRef) -> Self {
        self.key = Some(value);
        self
    }
}

impl StatelessWidget for ToggleSplitButton {
    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn build(&self, _app: &mut App, _context: BuildContext) -> WidgetRef {
        let next = !self.is_checked;
        let changed = self.is_checked_changed.clone();
        let click = self.click.clone();
        let button = SplitButton {
            content: self.content.clone(),
            click: Listener::new(move |app| {
                changed(app, next);
                if let Some(click) = &click {
                    click.call(app);
                }
            }),
            flyout: self.flyout,
            is_enabled: self.is_enabled,
            is_tab_stop: self.is_tab_stop,
            focus_node: self.focus_node,
            key: None,
        };
        SplitButtonPresenter {
            button,
            is_checked: self.is_checked,
        }
        .into_widget()
    }
}
