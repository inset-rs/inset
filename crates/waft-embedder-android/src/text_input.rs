//! The view's text input: Waft's text input view (`rs.waft.WaftTextInput`, which waft-cli builds
//! into the app and `WaftActivity` adds beside `NativeActivity`'s content view), which the system's
//! keyboards edit a field's text through, by the `InputConnection` it answers. The framework drives
//! it through `TextInputHost`, as Flutter's drives its Android `TextInputPlugin`; the field's
//! configuration is told to the keyboard as `editor_info` maps it.
//!
//! The field's text, selection and composing region live in that view, on Android's main thread,
//! where a keyboard calls and must be answered at once. This thread never reads or edits them:
//! what the framework asks (a field to start, its value, a stop) is handed to the view, which
//! carries it out on the main thread, and what the keyboard did comes back from the main thread
//! as [`TextInputEvent`]s, set down and woken for, which the loop hands the framework ([`take`]).
//! A later editor must keep both halves so: the main thread never calls the framework, and the
//! editing value has one owner.
//!
//! A key the host leaves unhandled reaches the view while a field takes text, which types it in;
//! a key the keyboard takes never reaches the host. Under an activity that is not Waft's there is
//! no such view, and no text input.

// JNI hands the native methods raw arguments, and registering them takes their functions as raw
// pointers; that is the `unsafe` here.
#![allow(unsafe_code)]

use std::sync::{LazyLock, Mutex, MutexGuard, PoisonError};

use android_activity::{AndroidApp, AndroidAppWaker};
use jni::errors::LogErrorAndDefault;
use jni::objects::{JObject, JValue};
use jni::sys::jint;
use jni::{Env, EnvUnowned, NativeMethod, jni_sig, jni_str};
use waft_embedder::{
    Matrix4, Rect, Size, TextEditingValue, TextInputAction, TextInputConfiguration, TextInputHost,
    TextRange, TextSelection,
};

use crate::activity::with_activity;
use crate::editor_info::{self, EditorInfo};
use crate::file_dialogs::string;
use crate::log::warn_to_log;

/// What the keyboard did since the loop last looked, in the order it did it.
static TYPED: LazyLock<Mutex<Vec<TextInputEvent>>> = LazyLock::new(Mutex::default);
/// Wakes the loop when the keyboard did something: the loop of the activity that set it last, since Android
/// starts another activity in the same process once one is finished.
static WAKER: Mutex<Option<AndroidAppWaker>> = Mutex::new(None);

/// Something the keyboard did to the field that takes text.
pub(crate) enum TextInputEvent {
    /// The field's value as the keyboard left it.
    Edited(TextEditingValue),
    /// The user pressed the field's action key.
    Action(TextInputAction),
}

/// What the keyboard did since the last call.
pub(crate) fn take() -> Vec<TextInputEvent> {
    std::mem::take(&mut *locked(&TYPED))
}

/// The activity's text input, through Waft's text input view.
pub(crate) struct AndroidTextInput {
    app: AndroidApp,
}

impl AndroidTextInput {
    /// The activity's text input, where the activity is Waft's and has its text input view: none
    /// under the bare `NativeActivity`, whose own view takes no text.
    pub(crate) fn new(app: &AndroidApp) -> Option<AndroidTextInput> {
        *locked(&WAKER) = Some(app.create_waker());
        let registered = with_activity(app, |env, activity| {
            let view = text_input_view(env, activity)?;
            register_native_methods(env, &view)
        });
        if registered.is_err() {
            clear_exception(app);
            return None;
        }
        Some(AndroidTextInput { app: app.clone() })
    }

    /// Calls `method` on the text input view, which carries it out on the main thread.
    fn tell_view(&self, method: &str, call: impl FnOnce(&mut Env<'_>, &JObject<'_>) -> CallResult) {
        let told = with_activity(&self.app, |env, activity| {
            let view = text_input_view(env, activity)?;
            call(env, &view)
        });
        if let Err(error) = told {
            clear_exception(&self.app);
            warn_to_log!("waft: the text input's {method} failed: {error}");
        }
    }
}

type CallResult = jni::errors::Result<()>;

impl TextInputHost for AndroidTextInput {
    /// Has the field take text, the keyboard told of it as `editor_info` maps its configuration,
    /// and shows the keyboard; for the field that takes text, shows the keyboard again.
    fn start(&self, configuration: &TextInputConfiguration) {
        let info = EditorInfo::of(configuration);
        self.tell_view("start", |env, view| {
            let label = match &info.action_label {
                Some(label) => env.new_string(label)?.into(),
                None => JObject::null(),
            };
            env.call_method(
                view,
                jni_str!("start"),
                jni_sig!("(IILjava/lang/String;)V"),
                &[
                    JValue::Int(info.input_type),
                    JValue::Int(info.ime_options),
                    JValue::Object(&label),
                ],
            )
            .map(drop)
        });
    }

    fn stop(&self) {
        self.tell_view("stop", |env, view| {
            env.call_method(view, jni_str!("stop"), jni_sig!("()V"), &[])
                .map(drop)
        });
    }

    fn set_editing_state(&self, value: &TextEditingValue) {
        self.tell_view("setEditingState", |env, view| {
            let text = env.new_string(&value.text)?;
            env.call_method(
                view,
                jni_str!("setEditingState"),
                jni_sig!("(Ljava/lang/String;IIII)V"),
                &[
                    JValue::Object(&text),
                    JValue::Int(value.selection.base_offset),
                    JValue::Int(value.selection.extent_offset),
                    JValue::Int(value.composing.start),
                    JValue::Int(value.composing.end),
                ],
            )
            .map(drop)
        });
    }

    /// Android's keyboards place nothing by the field's composing text.
    fn set_composing_rect(&self, _rect: Rect) {}

    /// Android's keyboards place nothing by the caret.
    fn set_caret_rect(&self, _rect: Rect) {}

    /// Android's keyboards place nothing by the field; Flutter's Android embedder keeps its rect
    /// only for autofill, which Waft does not have.
    fn set_client_geometry(&self, _size: Size, _transform: &Matrix4) {}
}

/// Waft's text input view, from the activity.
fn text_input_view<'local>(
    env: &mut Env<'local>,
    activity: &JObject<'_>,
) -> jni::errors::Result<JObject<'local>> {
    env.call_method(
        activity,
        jni_str!("textInput"),
        jni_sig!("()Lrs/waft/WaftTextInput;"),
        &[],
    )?
    .l()
}

/// Registers the view's `onEditingState` and `onEditorAction`, which it calls on the main thread.
fn register_native_methods(env: &mut Env<'_>, view: &JObject<'_>) -> CallResult {
    let class = env.get_object_class(view)?;
    let methods = [
        // SAFETY: `on_editing_state` takes the arguments `(Ljava/lang/String;IIII)V` names, as an
        // instance method's native function does.
        unsafe {
            NativeMethod::from_raw_parts(
                jni_str!("onEditingState"),
                jni_str!("(Ljava/lang/String;IIII)V"),
                on_editing_state as *mut std::ffi::c_void,
            )
        },
        // SAFETY: `on_editor_action` takes the arguments `(I)V` names, as an instance method's
        // native function does.
        unsafe {
            NativeMethod::from_raw_parts(
                jni_str!("onEditorAction"),
                jni_str!("(I)V"),
                on_editor_action as *mut std::ffi::c_void,
            )
        },
    ];
    // SAFETY: each method's function matches its signature, above.
    unsafe { env.register_native_methods(&class, &methods) }
}

/// What the view's `onEditingState` hands on, on the main thread: the field's value as the
/// keyboard left it, its offsets UTF-16's, as the framework's are.
extern "system" fn on_editing_state<'local>(
    mut env: EnvUnowned<'local>,
    _view: JObject<'local>,
    text: JObject<'local>,
    selection_start: jint,
    selection_end: jint,
    composing_start: jint,
    composing_end: jint,
) {
    env.with_env(|env| -> CallResult {
        let value = TextEditingValue {
            text: string(env, text)?.unwrap_or_default(),
            selection: TextSelection::new(selection_start, selection_end),
            composing: composing_range(composing_start, composing_end),
        };
        hand_on(TextInputEvent::Edited(value));
        Ok(())
    })
    .resolve::<LogErrorAndDefault>();
}

/// What the view's `onEditorAction` hands on, on the main thread.
extern "system" fn on_editor_action<'local>(
    mut env: EnvUnowned<'local>,
    _view: JObject<'local>,
    code: jint,
) {
    env.with_env(|_env| -> CallResult {
        hand_on(TextInputEvent::Action(editor_info::action_of(code)));
        Ok(())
    })
    .resolve::<LogErrorAndDefault>();
}

/// The composing region Android reports, -1 at both ends where there is none.
fn composing_range(start: jint, end: jint) -> TextRange {
    if start < 0 || end < start {
        TextRange::EMPTY
    } else {
        TextRange::new(start, end)
    }
}

fn hand_on(event: TextInputEvent) {
    locked(&TYPED).push(event);
    if let Some(waker) = locked(&WAKER).as_ref() {
        waker.wake();
    }
}

fn clear_exception(app: &AndroidApp) {
    let _ = with_activity(app, |env, _| {
        env.exception_clear();
        Ok(())
    });
}

fn locked<T>(shared: &Mutex<T>) -> MutexGuard<'_, T> {
    shared.lock().unwrap_or_else(PoisonError::into_inner)
}
