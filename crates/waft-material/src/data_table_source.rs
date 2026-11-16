//! Flutter counterpart: material_ui's `data_table_source.dart`.

use std::any::TypeId;
use std::fmt::{self, Debug};

use waft_foundation::{App, Handle, HandleId, Listenable, ListenableObject, Listener, Retained};

use crate::data_table::DataRow;

/// A data source for obtaining row data for [`PaginatedDataTable`](crate::PaginatedDataTable)
/// objects.
///
/// A data table source provides two main pieces of information:
///
///  * The number of rows in the data table ([`row_count`](Self::row_count)).
///  * The data for each row (indexed from `0` to `row_count - 1`).
///
/// It also provides some control over sort indicators and which rows are selected.
///
/// The data source is expected to expose some mechanism for sorting the data. To retrieve a
/// row's data, [`get_row`](Self::get_row) is called with the row's index. The data source
/// notifies its listeners whenever the data changes. It is the data source's responsibility to
/// be a `ChangeNotifier`, which gives it its [`ListenableObject`] side.
///
/// Dart's abstract `DataTableSource extends ChangeNotifier`: a data source is an arena object
/// that holds `ChangeNotifierData` and implements `ChangeNotifier`.
pub trait DataTableSource: ListenableObject {
    /// Called to obtain the data about a particular row.
    ///
    /// Rows should be keyed so that state can be maintained when the data source is sorted
    /// (e.g. with `DataRow::by_index`).
    ///
    /// Return `None` if the data for this row is unavailable (for example, if the row is
    /// still being loaded). The table will show a progress indicator for the first unavailable
    /// row of each page.
    ///
    /// If the given index is greater than or equal to [`row_count`](Self::row_count), this
    /// method should return `None` (but if [`is_row_count_approximate`](Self::is_row_count_approximate)
    /// is true, it may instead return a row for the data not yet loaded).
    fn get_row(self: Handle<Self>, app: &mut App, index: usize) -> Option<DataRow>;

    /// Called to obtain the number of rows to tell the user are available.
    ///
    /// If [`is_row_count_approximate`](Self::is_row_count_approximate) is false, then this
    /// must be an accurate number, and [`get_row`](Self::get_row) must return a non-null value
    /// for all indices in the range 0 to one less than the row count.
    ///
    /// If [`is_row_count_approximate`](Self::is_row_count_approximate) is true, then this must
    /// be an estimate.
    fn row_count(self: Handle<Self>, app: &App) -> usize;

    /// Called to establish if [`row_count`](Self::row_count) is a precise number or might be
    /// an over-estimate. If this returns true (i.e. the count is approximate), and then later
    /// the exact number becomes available, then call `notify_listeners`.
    fn is_row_count_approximate(self: Handle<Self>, app: &App) -> bool;

    /// Called to obtain the number of rows that are currently selected.
    ///
    /// If the selected row count changes, call `notify_listeners`.
    ///
    /// Selected rows are those whose [`DataRow::selected`] property is set to true.
    fn selected_row_count(self: Handle<Self>, app: &App) -> usize;

    /// The type-erased `DataTableSource` handle. Free: the vtable is a `const`, and the id is
    /// copied.
    fn as_data_table_source(self: Handle<Self>) -> AnyDataTableSource {
        AnyDataTableSource {
            id: self.id(),
            vtable: const { &DataTableSourceVTable::of::<Self>() },
        }
    }
}

/// The vtable of an erased [`AnyDataTableSource`]: one `&'static` table per concrete
/// [`DataTableSource`] type.
struct DataTableSourceVTable {
    type_name: fn() -> &'static str,
    add_listener: fn(&mut App, HandleId, Listener),
    remove_listener: fn(&mut App, HandleId, &Listener),
    get_row: fn(&mut App, HandleId, usize) -> Option<DataRow>,
    row_count: fn(&App, HandleId) -> usize,
    is_row_count_approximate: fn(&App, HandleId) -> bool,
    selected_row_count: fn(&App, HandleId) -> usize,
    runtime_type: fn() -> TypeId,
}

impl DataTableSourceVTable {
    const fn of<T: DataTableSource>() -> DataTableSourceVTable {
        DataTableSourceVTable {
            type_name: std::any::type_name::<T>,
            runtime_type: TypeId::of::<T>,
            add_listener: |app, id, listener| {
                ListenableObject::add_listener(Handle::<T>::from_id(id), app, listener)
            },
            remove_listener: |app, id, listener| {
                ListenableObject::remove_listener(Handle::<T>::from_id(id), app, listener)
            },
            get_row: |app, id, index| T::get_row(Handle::<T>::from_id(id), app, index),
            row_count: |app, id| T::row_count(Handle::<T>::from_id(id), app),
            is_row_count_approximate: |app, id| {
                T::is_row_count_approximate(Handle::<T>::from_id(id), app)
            },
            selected_row_count: |app, id| T::selected_row_count(Handle::<T>::from_id(id), app),
        }
    }
}

/// Erased [`DataTableSource`]: what a field or parameter Dart types as `DataTableSource`
/// becomes.
///
/// Equality is Dart's `==` on an object reference.
#[derive(Clone, Copy)]
pub struct AnyDataTableSource {
    id: HandleId,
    vtable: &'static DataTableSourceVTable,
}

/// A field Dart types as the erased object, kept: [`AnyDataTableSource`] with its count.
pub type DataTableSourceRef = Retained<AnyDataTableSource>;

waft_foundation::erased_handle!(AnyDataTableSource: DataTableSource => DataTableSource::as_data_table_source);

impl AnyDataTableSource {
    /// The arena id behind this handle.
    pub fn id(self) -> HandleId {
        self.id
    }

    /// Dart's `runtimeType`.
    pub fn runtime_type(self) -> TypeId {
        (self.vtable.runtime_type)()
    }

    /// Dart's `source as S`: the typed handle when this source is an `S`, else `None`.
    pub fn downcast<S: 'static>(self, app: &App) -> Option<Handle<S>> {
        app.handle::<S>(self.id)
    }

    /// See [`DataTableSource::get_row`].
    pub fn get_row(self, app: &mut App, index: usize) -> Option<DataRow> {
        (self.vtable.get_row)(app, self.id, index)
    }

    /// See [`DataTableSource::row_count`].
    pub fn row_count(self, app: &App) -> usize {
        (self.vtable.row_count)(app, self.id)
    }

    /// See [`DataTableSource::is_row_count_approximate`].
    pub fn is_row_count_approximate(self, app: &App) -> bool {
        (self.vtable.is_row_count_approximate)(app, self.id)
    }

    /// See [`DataTableSource::selected_row_count`].
    pub fn selected_row_count(self, app: &App) -> usize {
        (self.vtable.selected_row_count)(app, self.id)
    }
}

impl Debug for AnyDataTableSource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}({:?})", (self.vtable.type_name)(), self.id)
    }
}

impl Listenable for AnyDataTableSource {
    fn add_listener(&self, app: &mut App, listener: Listener) {
        (self.vtable.add_listener)(app, self.id, listener)
    }

    fn remove_listener(&self, app: &mut App, listener: &Listener) {
        (self.vtable.remove_listener)(app, self.id, listener)
    }
}
