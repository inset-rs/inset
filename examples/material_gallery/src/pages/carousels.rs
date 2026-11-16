//! Carousels: `CarouselView` multi-browse with a fixed item extent, hero with weights, and
//! uncontained with no snapping. A caption under each says which item was tapped last.

use waft_embedder::Color;
use waft_foundation::{App, Handle};
use waft_material::{CarouselView, Colors};

use waft_widgets::{
    BuildContext, Center, ColoredBox, IntoWidget, ListView, SizedBox, State, StateData,
    StatefulWidget, Text, WidgetRef,
};

use crate::catalog::Entry;
use crate::support::{caption, page, section_header};

const ITEM_COUNT: usize = 8;

pub fn body(_app: &mut App, _context: BuildContext) -> WidgetRef {
    page(Entry::Carousels.title(), CarouselsDemo.into_widget()).into_widget()
}

/// The last tapped item of each carousel lives here, in the smallest widget that renders it.
#[derive(Debug)]
pub struct CarouselsDemo;

/// Dart would spell this `_CarouselsDemoState`.
pub struct CarouselsDemoState {
    state: StateData<CarouselsDemo>,
    last_tapped: [Option<i32>; 3],
}

impl StatefulWidget for CarouselsDemo {
    type State = CarouselsDemoState;

    fn create_state(&self) -> CarouselsDemoState {
        CarouselsDemoState {
            state: StateData::new(),
            last_tapped: [None; 3],
        }
    }
}

impl State for CarouselsDemoState {
    type Widget = CarouselsDemo;
    waft_widgets::state_accessors!();

    fn build(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        let last_tapped = app.get(self).last_tapped;
        let shown = |index: usize| match last_tapped[index] {
            Some(item) => format!("Tapped item {item}"),
            None => "Nothing tapped yet".to_owned(),
        };
        let carousel = |index: usize, view: CarouselView| {
            SizedBox::new()
                .height(200.0)
                .child(
                    view.on_tap(std::rc::Rc::new(move |app: &mut App, item: i32| {
                        self.set_state(app, |state| state.last_tapped[index] = Some(item));
                    })),
                )
                .into_widget()
        };
        ListView::new()
            .children(vec![
                section_header(app, context, "Multi-browse"),
                caption(app, context, shown(0)),
                carousel(0, CarouselView::new(186.0, items())),
                section_header(app, context, "Hero"),
                caption(app, context, shown(1)),
                carousel(1, CarouselView::weighted(vec![1, 7, 1], items())),
                section_header(app, context, "Uncontained"),
                caption(app, context, shown(2)),
                carousel(2, CarouselView::new(150.0, items()).item_snapping(false)),
            ])
            .into_widget()
    }
}

/// Eight coloured tiles, each numbered.
fn items() -> Vec<WidgetRef> {
    (0..ITEM_COUNT)
        .map(|index| {
            let palette = [
                Colors::INDIGO.primary(),
                Colors::TEAL.primary(),
                Colors::ORANGE.primary(),
                Colors::PINK.primary(),
            ];
            let color: Color = palette[index % palette.len()];
            ColoredBox::new(color)
                .child(Center::new().child(Text::new(format!("Item {index}"))))
                .into_widget()
        })
        .collect()
}
