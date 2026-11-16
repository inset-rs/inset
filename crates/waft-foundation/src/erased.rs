//! What every erased handle shares: Dart's abstract class used as a type is one `Copy` struct
//! `AnyFoo { id: HandleId, vtable }` per class, and [`erased_handle!`](crate::erased_handle)
//! writes the items that are the same for all of them. A type's own members, its `Debug` and its
//! `downcast` stay with the type.

/// The items every erased handle has, for `AnyFoo { id: HandleId, .. }` whose leaves implement
/// `FooLeaf` and reach it through the function `upcast` (the trait method that defines it,
/// such as `FooLeaf::as_foo`):
///
/// - `From<AnyFoo> for HandleId`, and equality and hashing by the object's id, as Dart's `==`
///   on an object reference;
/// - equality with a [`Retained`](crate::Retained) of the same object;
/// - `From<Handle<T>>` for every leaf `T`, so a typed [`Ref`](crate::Ref) erases with
///   [`Retained::erase`](crate::Retained::erase): Dart's upcast;
/// - `upgrade` and `keep`, as [`Handle::upgrade`](crate::Handle::upgrade) and
///   [`Handle::keep`](crate::Handle::keep) are a typed handle's.
///
/// ```ignore
/// waft_foundation::erased_handle!(AnyFocusNode: FocusNodeLeaf => FocusNodeLeaf::as_node);
/// waft_foundation::erased_handle!(AnyAnimation<T>: Animation => Animation::as_animation);
/// ```
///
/// A generic erased handle names its one type parameter, which the leaf trait takes too.
#[macro_export]
macro_rules! erased_handle {
    ($erased:ident : $leaf:path => $upcast:path) => {
        impl ::core::convert::From<$erased> for $crate::HandleId {
            fn from(handle: $erased) -> $crate::HandleId {
                handle.id
            }
        }

        impl ::core::cmp::PartialEq for $erased {
            fn eq(&self, other: &$erased) -> bool {
                self.id == other.id
            }
        }

        impl ::core::cmp::Eq for $erased {}

        impl ::core::hash::Hash for $erased {
            fn hash<H: ::core::hash::Hasher>(&self, state: &mut H) {
                ::core::hash::Hash::hash(&self.id, state);
            }
        }

        impl ::core::cmp::PartialEq<$crate::Retained<$erased>> for $erased {
            fn eq(&self, other: &$crate::Retained<$erased>) -> bool {
                self.id == other.handle().id
            }
        }

        impl<Leaf: $leaf> ::core::convert::From<$crate::Handle<Leaf>> for $erased {
            fn from(handle: $crate::Handle<Leaf>) -> $erased {
                $upcast(handle)
            }
        }

        impl $erased {
            /// A reference while something still keeps the object, `None` once it is freed or
            /// its last keeper has let go; see [`Handle::upgrade`]($crate::Handle::upgrade).
            pub fn upgrade(
                self,
                app: &$crate::App,
            ) -> ::core::option::Option<$crate::Retained<$erased>> {
                $crate::Retained::upgrade(app, self)
            }

            /// A reference to an object something is known to keep; see
            /// [`Handle::keep`]($crate::Handle::keep).
            #[track_caller]
            pub fn keep(self, app: &$crate::App) -> $crate::Retained<$erased> {
                $crate::Retained::keep(app, self)
            }
        }
    };
    ($erased:ident<$param:ident> : $leaf:ident => $upcast:path) => {
        impl<$param: 'static> ::core::convert::From<$erased<$param>> for $crate::HandleId {
            fn from(handle: $erased<$param>) -> $crate::HandleId {
                handle.id
            }
        }

        impl<$param: 'static> ::core::cmp::PartialEq for $erased<$param> {
            fn eq(&self, other: &$erased<$param>) -> bool {
                self.id == other.id
            }
        }

        impl<$param: 'static> ::core::cmp::Eq for $erased<$param> {}

        impl<$param: 'static> ::core::hash::Hash for $erased<$param> {
            fn hash<H: ::core::hash::Hasher>(&self, state: &mut H) {
                ::core::hash::Hash::hash(&self.id, state);
            }
        }

        impl<$param: 'static> ::core::cmp::PartialEq<$crate::Retained<$erased<$param>>>
            for $erased<$param>
        {
            fn eq(&self, other: &$crate::Retained<$erased<$param>>) -> bool {
                self.id == other.handle().id
            }
        }

        impl<$param: 'static, Leaf: $leaf<$param>> ::core::convert::From<$crate::Handle<Leaf>>
            for $erased<$param>
        {
            fn from(handle: $crate::Handle<Leaf>) -> $erased<$param> {
                $upcast(handle)
            }
        }

        impl<$param: 'static> $erased<$param> {
            /// A reference while something still keeps the object, `None` once it is freed or
            /// its last keeper has let go; see [`Handle::upgrade`]($crate::Handle::upgrade).
            pub fn upgrade(
                self,
                app: &$crate::App,
            ) -> ::core::option::Option<$crate::Retained<$erased<$param>>> {
                $crate::Retained::upgrade(app, self)
            }

            /// A reference to an object something is known to keep; see
            /// [`Handle::keep`]($crate::Handle::keep).
            #[track_caller]
            pub fn keep(self, app: &$crate::App) -> $crate::Retained<$erased<$param>> {
                $crate::Retained::keep(app, self)
            }
        }
    };
}
