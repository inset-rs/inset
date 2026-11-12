//! A view's text input: Waft's text input view (`rs.waft.WaftTextInput`, which waft-cli builds
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
//! Each activity has a text input view of its own, and the native methods are the process's: so
//! each view is given the id of the host's view it types into ([`AndroidTextInput::new`]) and
//! hands it back with everything it tells, as the Java side of Flutter's embedding carries its
//! shell's id into each native call (`FlutterJNI.nativeShellHolderId`). Flutter has no counterpart
//! for the rest, since its engine has one activity at a time: what the keyboards did is one queue
//! for the app, in the order they did it, each event naming its view, and it wakes the app's one
//! loop. A later editor must not key anything here by "the" activity.
//!
//! A key the host leaves unhandled reaches the view while a field takes text, which types it in;
//! a key the keyboard takes never reaches the host. Under an activity that is not Waft's there is
//! no such view, and no text input.

// JNI hands the native methods raw arguments, and registering them takes their functions as raw
// pointers; that is the `unsafe` here.
#![allow(unsafe_code)]

use std::sync::{LazyLock, Mutex, MutexGuard, OnceLock, PoisonError};

use jni::errors::LogErrorAndDefault;
use jni::objects::{JObject, JValue};
use jni::sys::jint;
use jni::sys::jlong;
use jni::{Env, EnvUnowned, NativeMethod, jni_sig, jni_str};
use waft_embedder::{
    Matrix4, Rect, Size, TextEditingValue, TextInputAction, TextInputConfiguration, TextInputHost,
    TextRange, TextSelection, ViewId,
};

use crate::activity::Activity;
use crate::application::Application;
use crate::editor_info::{self, EditorInfo};
use crate::file_dialogs::string;
use crate::log::warn_to_log;
use crate::loop_waker::wake_app_loop;

/// Waft's text input view, which declares the native methods registered here.
const TEXT_INPUT_VIEW: &str = "rs.waft.WaftTextInput";

/// What the keyboards did since the loop last looked, in the order they did it, each to the field
/// of the view it names.
static TYPED: LazyLock<Mutex<Vec<(ViewId, TextInputEvent)>>> = LazyLock::new(Mutex::default);

/// Something a keyboard did to the field that takes text in a view.
pub(crate) enum TextInputEvent {
    /// The field's value as the keyboard left it.
    Edited(TextEditingValue),
    /// The user pressed the field's action key.
    Action(TextInputAction),
}

/// What the keyboards did since the last call, and in which view.
pub(crate) fn take() -> Vec<(ViewId, TextInputEvent)> {
    std::mem::take(&mut *locked(&TYPED))
}

/// An activity's text input, through Waft's text input view.
pub(crate) struct AndroidTextInput {
    activity: Activity,
}

impl AndroidTextInput {
    /// The text input of `view`, the host's view `activity`'s window is, where the activity is
    /// Waft's and has its text input view: none under the bare `NativeActivity`, whose own view
    /// takes no text. What a keyboard does there is told as done to `view` from now on.
    pub(crate) fn new(
        application: &Application,
        activity: &Activity,
        view: ViewId,
    ) -> Option<AndroidTextInput> {
        if !the_keyboard_is_heard(application) {
            return None;
        }
        // A view before this one, in the same process, may have left what was typed into it.
        locked(&TYPED).retain(|(typed_into, _)| *typed_into != view);
        let attached = activity.with(|env, activity| {
            let text_input = text_input_view(env, activity)?;
            attach(env, &text_input, view)
        });
        if attached.is_err() {
            activity.clear_exception();
            return None;
        }
        Some(AndroidTextInput {
            activity: activity.clone(),
        })
    }

    /// Calls `method` on the text input view, which carries it out on the main thread.
    fn tell_view(&self, method: &str, call: impl FnOnce(&mut Env<'_>, &JObject<'_>) -> CallResult) {
        let told = self.activity.with(|env, activity| {
            let view = text_input_view(env, activity)?;
            call(env, &view)
        });
        if let Err(error) = told {
            self.activity.clear_exception();
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

/// Gives Waft's text input view the id of the host's view it types into, which it hands back with
/// what it tells.
fn attach(env: &mut Env<'_>, text_input: &JObject<'_>, view: ViewId) -> CallResult {
    env.call_method(
        text_input,
        jni_str!("attach"),
        jni_sig!("(J)V"),
        &[JValue::Long(view.0 as jlong)],
    )
    .map(drop)
}

/// Registers the text input view's `onEditingState` and `onEditorAction`, which a view calls on
/// the main thread, once for the process, and answers whether they are registered: not in a
/// package without the view.
fn the_keyboard_is_heard(application: &Application) -> bool {
    static HEARD: OnceLock<bool> = OnceLock::new();
    *HEARD.get_or_init(|| register_native_methods(application).is_ok())
}

fn register_native_methods(application: &Application) -> CallResult {
    let methods = [
        // SAFETY: `on_editing_state` takes the arguments `(JLjava/lang/String;IIII)V` names, as an
        // instance method's native function does.
        unsafe {
            NativeMethod::from_raw_parts(
                jni_str!("onEditingState"),
                jni_str!("(JLjava/lang/String;IIII)V"),
                on_editing_state as *mut std::ffi::c_void,
            )
        },
        // SAFETY: `on_editor_action` takes the arguments `(JI)V` names, as an instance method's
        // native function does.
        unsafe {
            NativeMethod::from_raw_parts(
                jni_str!("onEditorAction"),
                jni_str!("(JI)V"),
                on_editor_action as *mut std::ffi::c_void,
            )
        },
    ];
    // SAFETY: each method's function matches its signature, above.
    unsafe { application.register_native_methods(TEXT_INPUT_VIEW, &methods) }
}

/// What a text input view's `onEditingState` hands on, on the main thread: the value of the field
/// of the view numbered `view` as the keyboard left it, its offsets UTF-16's, as the framework's
/// are.
extern "system" fn on_editing_state<'local>(
    mut env: EnvUnowned<'local>,
    _text_input: JObject<'local>,
    view: jlong,
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
        hand_on(view, TextInputEvent::Edited(value));
        Ok(())
    })
    .resolve::<LogErrorAndDefault>();
}

/// What a text input view's `onEditorAction` hands on, on the main thread, for the field of the
/// view numbered `view`.
extern "system" fn on_editor_action<'local>(
    mut env: EnvUnowned<'local>,
    _text_input: JObject<'local>,
    view: jlong,
    code: jint,
) {
    env.with_env(|_env| -> CallResult {
        hand_on(view, TextInputEvent::Action(editor_info::action_of(code)));
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

/// Sets `event` down for the loop as done in the view numbered `view`, and wakes it.
fn hand_on(view: jlong, event: TextInputEvent) {
    locked(&TYPED).push((ViewId(view as u64), event));
    wake_app_loop();
}

fn locked<T>(shared: &Mutex<T>) -> MutexGuard<'_, T> {
    shared.lock().unwrap_or_else(PoisonError::into_inner)
}
