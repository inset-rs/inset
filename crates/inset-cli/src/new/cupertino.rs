//! The Cupertino template: a page scaffold with one line of text.

use super::Kit;

pub const KIT: Kit = Kit {
    dependencies: "inset-cupertino = \"{inset}\"\n",
    lib_rs: r#"// StatefulWidget::State takes `self: Handle<Self>`.
#![feature(arbitrary_self_types)]

use inset::{App, BuildContext, Center, IntoWidget, StatelessWidget, Text, WidgetRef, run_app};
use inset_cupertino::{CupertinoApp, CupertinoPageScaffold};

#[inset::main]
fn main(app: &mut App) {
    run_app(app, CupertinoApp::new().home(Home).into_widget());
}

#[derive(Debug)]
struct Home;

impl StatelessWidget for Home {
    fn build(&self, _app: &mut App, _context: BuildContext) -> WidgetRef {
        CupertinoPageScaffold::new(Center::new().child(Text::new("Hello, Inset"))).into_widget()
    }
}
"#,
};
