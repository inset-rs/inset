package rs.waft;

import android.content.Context;
import android.icu.text.BreakIterator;
import android.os.Handler;
import android.os.Looper;
import android.text.DynamicLayout;
import android.text.Editable;
import android.text.InputType;
import android.text.Layout;
import android.text.Selection;
import android.text.SpannableStringBuilder;
import android.text.TextPaint;
import android.view.KeyCharacterMap;
import android.view.KeyEvent;
import android.view.View;
import android.view.inputmethod.BaseInputConnection;
import android.view.inputmethod.EditorInfo;
import android.view.inputmethod.ExtractedText;
import android.view.inputmethod.ExtractedTextRequest;
import android.view.inputmethod.InputConnection;
import android.view.inputmethod.InputMethodManager;
import java.util.Objects;

/**
 * Waft's text input: the view the system's keyboards edit a field's text through. A keyboard edits
 * text only through the {@link InputConnection} of a focused view, which NativeActivity's own
 * content view does not give, so {@link WaftActivity} adds this one beside it. It draws nothing and
 * takes no touches: the window's surface and its input stay NativeActivity's.
 *
 * <p>The field's text, its selection and its composing region live here, in {@link #editable}, on
 * the main thread, where the keyboard reads and edits them and must be answered at once. The app's
 * native code runs on a thread of its own: it starts a field, sets its value and stops it through
 * {@link #start}, {@link #setEditingState} and {@link #stop}, which carry each out on the main
 * thread, and hears each change the keyboard made, once its batch of edits ends, and each action
 * key, through {@link #onEditingState} and {@link #onEditorAction}. Every change to the editable
 * is made inside {@link #beginEdit} and {@link #endEdit}, which tell both sides. This is the half
 * of Flutter's Android {@code TextInputPlugin} and {@code InputConnectionAdaptor} that is Java's.
 *
 * <p>A key the app's native code leaves unhandled reaches this view while it has the focus: while a
 * field takes text, it types into it ({@link #type}), as Flutter's {@code handleKeyEvent} does for a
 * key its framework left. A key the keyboard takes, it edits through the connection instead, and
 * neither the native code nor this view sees it.
 *
 * <p>The app's native code is one for the process, whichever activities it has, and each
 * activity's window is one of its views: the native code gives this the number of its view
 * ({@link #attach}) before it starts a field, and everything it is told carries it, as the Java
 * side of Flutter's embedding carries its shell's id into each native call ({@code
 * FlutterJNI.nativeShellHolderId}). The native methods are registered by the app's native code
 * before it attaches a view. waft-cli compiles this class into every Android app.
 */
final class WaftTextInput extends View {
    /** No view: the native code has not attached this yet. */
    private static final long NO_VIEW = -1;

    private final InputMethodManager inputMethods;
    private final Handler mainThread = new Handler(Looper.getMainLooper());
    /** The number the native code gave the view this types into; the main thread's alone. */
    private long view = NO_VIEW;
    /** The field's text, its selection and its composing region; the main thread's alone. */
    private final SpannableStringBuilder editable = new SpannableStringBuilder();
    /** Sets the editable's composing region, which only a connection can. */
    private final BaseInputConnection composer;
    /** The editable's lines, for the arrow keys that move a line up or down. */
    private final Layout lines;
    /** What the keyboard is told of the field that takes text; null while none does. */
    private Field field;
    private int editDepth;
    /** The editable as the outermost edit under way found it. */
    private State editStart;
    /** The state both sides last agreed on: the last the native code set or was told. */
    private State known = State.NONE;
    /** The keyboard's request to hear the text as it changes, while it monitors it. */
    private ExtractedTextRequest extractMonitor;

    WaftTextInput(Context context) {
        super(context);
        setFocusable(true);
        setFocusableInTouchMode(true);
        inputMethods = context.getSystemService(InputMethodManager.class);
        composer =
                new BaseInputConnection(this, true) {
                    @Override
                    public Editable getEditable() {
                        return editable;
                    }
                };
        lines = DynamicLayout.Builder.obtain(editable, new TextPaint(), Integer.MAX_VALUE).build();
    }

    /**
     * What the keyboard does here is told as done to the view numbered {@code view} from now on.
     * Called by the app's native code, from its thread, before it starts a field.
     */
    void attach(long view) {
        mainThread.post(() -> this.view = view);
    }

    /**
     * Has a field configured so take text, and shows the keyboard for it; again for the field that
     * takes text, it shows the keyboard again. Called by the app's native code, from its thread.
     */
    void start(int inputType, int imeOptions, String actionLabel) {
        Field started = new Field(inputType, imeOptions, actionLabel);
        mainThread.post(() -> startField(started));
    }

    /** No field takes text any more: the keyboard hides. Called from the native code's thread. */
    void stop() {
        mainThread.post(this::stopField);
    }

    /** The field's value as the framework set it. Called from the native code's thread. */
    void setEditingState(
            String text, int selectionStart, int selectionEnd, int composingStart, int composingEnd) {
        State state = new State(text, selectionStart, selectionEnd, composingStart, composingEnd);
        mainThread.post(() -> setState(state));
    }

    private void startField(Field started) {
        boolean newField = !started.equals(field);
        field = started;
        requestFocus();
        // The keyboard reads the field's configuration only as it connects.
        if (newField) {
            inputMethods.restartInput(this);
        }
        if (started.inputType == InputType.TYPE_NULL) {
            hideKeyboard();
        } else {
            inputMethods.showSoftInput(this, 0);
        }
    }

    private void stopField() {
        field = null;
        hideKeyboard();
        inputMethods.restartInput(this);
    }

    private void hideKeyboard() {
        inputMethods.hideSoftInputFromWindow(getWindowToken(), 0);
    }

    /**
     * Takes the framework's value. Where it replaced the text or the composing region the keyboard
     * holds, the keyboard connects again, as it does to an {@code EditText} whose text is set; a
     * selection alone it is told of.
     */
    private void setState(State state) {
        State current = State.of(editable);
        boolean replaced =
                !state.text.equals(current.text)
                        || state.composingStart != current.composingStart
                        || state.composingEnd != current.composingEnd;
        known = state;
        beginEdit();
        if (!state.text.equals(current.text)) {
            editable.replace(0, editable.length(), state.text);
        }
        if (state.selectionStart >= 0 && state.selectionEnd >= 0) {
            Selection.setSelection(
                    editable, clamped(state.selectionStart), clamped(state.selectionEnd));
        } else {
            Selection.removeSelection(editable);
        }
        if (state.composingStart >= 0 && state.composingStart < state.composingEnd) {
            composer.setComposingRegion(state.composingStart, state.composingEnd);
        } else {
            BaseInputConnection.removeComposingSpans(editable);
        }
        endEdit();
        if (replaced && field != null) {
            inputMethods.restartInput(this);
        }
    }

    private int clamped(int offset) {
        return Math.max(0, Math.min(editable.length(), offset));
    }

    private void beginEdit() {
        if (editDepth++ == 0) {
            editStart = State.of(editable);
        }
    }

    /**
     * Ends an edit; the outermost tells the keyboard where the selection and the composing region
     * now are, if they changed, and the native code the value, if it is not the one both know.
     */
    private void endEdit() {
        if (editDepth == 0 || --editDepth > 0) {
            return;
        }
        State now = State.of(editable);
        if (!now.equals(editStart)) {
            tellKeyboard(now);
        }
        if (field != null && !now.equals(known)) {
            known = now;
            onEditingState(
                    view,
                    now.text,
                    now.selectionStart,
                    now.selectionEnd,
                    now.composingStart,
                    now.composingEnd);
        }
    }

    private void tellKeyboard(State now) {
        inputMethods.updateSelection(
                this, now.selectionStart, now.selectionEnd, now.composingStart, now.composingEnd);
        if (extractMonitor != null) {
            inputMethods.updateExtractedText(this, extractMonitor.token, extracted());
        }
    }

    private ExtractedText extracted() {
        ExtractedText extracted = new ExtractedText();
        extracted.text = editable.toString();
        extracted.partialStartOffset = -1;
        extracted.partialEndOffset = -1;
        extracted.selectionStart = Selection.getSelectionStart(editable);
        extracted.selectionEnd = Selection.getSelectionEnd(editable);
        return extracted;
    }

    @Override
    public boolean onCheckIsTextEditor() {
        return field != null;
    }

    @Override
    public InputConnection onCreateInputConnection(EditorInfo info) {
        if (field == null) {
            return null;
        }
        info.inputType = field.inputType;
        info.imeOptions = field.imeOptions;
        if (field.actionLabel != null) {
            info.actionLabel = field.actionLabel;
            info.actionId = field.imeOptions & EditorInfo.IME_MASK_ACTION;
        }
        info.initialSelStart = Selection.getSelectionStart(editable);
        info.initialSelEnd = Selection.getSelectionEnd(editable);
        info.setInitialSurroundingText(editable);
        return new Connection();
    }

    @Override
    public boolean onKeyDown(int keyCode, KeyEvent event) {
        return (field != null && type(event)) || super.onKeyDown(keyCode, event);
    }

    /**
     * What a key does to the field that takes text: an arrow moves the caret, or extends the
     * selection with shift; delete and forward delete take away the selection or the character
     * beside the caret; enter in a field of one line is its action; any other key with a character
     * types it. The framework sees no key on Android yet, so delete is here, where Flutter's
     * framework takes it first.
     */
    private boolean type(KeyEvent event) {
        switch (event.getKeyCode()) {
            case KeyEvent.KEYCODE_DPAD_LEFT:
                return moveAcross(true, event.isShiftPressed());
            case KeyEvent.KEYCODE_DPAD_RIGHT:
                return moveAcross(false, event.isShiftPressed());
            case KeyEvent.KEYCODE_DPAD_UP:
                return moveLine(true, event.isShiftPressed());
            case KeyEvent.KEYCODE_DPAD_DOWN:
                return moveLine(false, event.isShiftPressed());
            case KeyEvent.KEYCODE_DEL:
                return delete(true);
            case KeyEvent.KEYCODE_FORWARD_DEL:
                return delete(false);
            case KeyEvent.KEYCODE_ENTER:
            case KeyEvent.KEYCODE_NUMPAD_ENTER:
                if ((field.inputType & InputType.TYPE_TEXT_FLAG_MULTI_LINE) == 0) {
                    onEditorAction(view, field.imeOptions & EditorInfo.IME_MASK_ACTION);
                    return true;
                }
                return insert(event.getUnicodeChar());
            default:
                return insert(event.getUnicodeChar());
        }
    }

    private boolean moveAcross(boolean left, boolean extend) {
        int start = Selection.getSelectionStart(editable);
        int end = Selection.getSelectionEnd(editable);
        if (start < 0 || end < 0) {
            return false;
        }
        int moved = left ? characterBefore(end) : characterAfter(end);
        beginEdit();
        Selection.setSelection(editable, extend ? start : moved, moved);
        endEdit();
        return true;
    }

    private boolean moveLine(boolean up, boolean extend) {
        if (Selection.getSelectionStart(editable) < 0) {
            return false;
        }
        beginEdit();
        if (extend) {
            if (up) Selection.extendUp(editable, lines);
            else Selection.extendDown(editable, lines);
        } else {
            if (up) Selection.moveUp(editable, lines);
            else Selection.moveDown(editable, lines);
        }
        endEdit();
        return true;
    }

    private boolean delete(boolean backward) {
        int start = Selection.getSelectionStart(editable);
        int end = Selection.getSelectionEnd(editable);
        if (start < 0 || end < 0) {
            return false;
        }
        int from = Math.min(start, end);
        int to = Math.max(start, end);
        if (from == to) {
            if (backward) from = characterBefore(to);
            else to = characterAfter(from);
        }
        beginEdit();
        editable.delete(from, to);
        Selection.setSelection(editable, from);
        endEdit();
        return true;
    }

    private boolean insert(int character) {
        int start = Selection.getSelectionStart(editable);
        int end = Selection.getSelectionEnd(editable);
        // A dead key's accent combines with the next key, which this view does not compose.
        if (character == 0 || (character & KeyCharacterMap.COMBINING_ACCENT) != 0) {
            return false;
        }
        if (start < 0 || end < 0) {
            return false;
        }
        String typed = new String(Character.toChars(character));
        int from = Math.min(start, end);
        beginEdit();
        editable.replace(from, Math.max(start, end), typed);
        Selection.setSelection(editable, from + typed.length());
        endEdit();
        return true;
    }

    /** Where the character, a user-perceived one, before {@code offset} starts. */
    private int characterBefore(int offset) {
        BreakIterator characters = BreakIterator.getCharacterInstance();
        characters.setText(editable.toString());
        int before = characters.preceding(Math.min(offset, editable.length()));
        return before == BreakIterator.DONE ? 0 : before;
    }

    /** Where the character, a user-perceived one, after {@code offset} ends. */
    private int characterAfter(int offset) {
        BreakIterator characters = BreakIterator.getCharacterInstance();
        characters.setText(editable.toString());
        int after = characters.following(Math.max(offset, 0));
        return after == BreakIterator.DONE ? editable.length() : after;
    }

    /**
     * Hands the app's native code the value the keyboard changed, of the field of the view numbered
     * {@code view}; registered by it.
     */
    private native void onEditingState(
            long view,
            String text,
            int selectionStart,
            int selectionEnd,
            int composingStart,
            int composingEnd);

    /**
     * Hands the app's native code an action key the user pressed in the field of the view numbered
     * {@code view}; registered by it.
     */
    private native void onEditorAction(long view, int action);

    /**
     * The connection a keyboard edits the field through: the platform's own over the editable, its
     * batches of edits being this view's, so the field's value is told once a batch ends.
     */
    private final class Connection extends BaseInputConnection {
        /** The batches this connection began and has not ended. */
        private int batches;

        Connection() {
            super(WaftTextInput.this, true);
        }

        @Override
        public Editable getEditable() {
            return editable;
        }

        @Override
        public boolean beginBatchEdit() {
            batches++;
            beginEdit();
            return true;
        }

        @Override
        public boolean endBatchEdit() {
            if (batches == 0) {
                return false;
            }
            batches--;
            endEdit();
            return batches > 0;
        }

        @Override
        public void closeConnection() {
            super.closeConnection();
            while (batches > 0) {
                endBatchEdit();
            }
        }

        @Override
        public boolean setSelection(int start, int end) {
            beginBatchEdit();
            boolean set = super.setSelection(start, end);
            endBatchEdit();
            return set;
        }

        /** An empty composition is committed, so no empty composing region is left behind. */
        @Override
        public boolean setComposingText(CharSequence text, int newCursorPosition) {
            beginBatchEdit();
            boolean set =
                    text.length() == 0
                            ? super.commitText(text, newCursorPosition)
                            : super.setComposingText(text, newCursorPosition);
            endBatchEdit();
            return set;
        }

        @Override
        public boolean deleteSurroundingText(int beforeLength, int afterLength) {
            if (Selection.getSelectionStart(editable) < 0) {
                return true;
            }
            return super.deleteSurroundingText(beforeLength, afterLength);
        }

        @Override
        public boolean performEditorAction(int action) {
            if (field != null) {
                onEditorAction(view, action);
            }
            return true;
        }

        @Override
        public ExtractedText getExtractedText(ExtractedTextRequest request, int flags) {
            extractMonitor = (flags & GET_EXTRACTED_TEXT_MONITOR) != 0 ? request : null;
            return extracted();
        }
    }

    /** What the keyboard is told of a field: its {@link EditorInfo}'s part the app decides. */
    private static final class Field {
        final int inputType;
        final int imeOptions;
        final String actionLabel;

        Field(int inputType, int imeOptions, String actionLabel) {
            this.inputType = inputType;
            this.imeOptions = imeOptions;
            this.actionLabel = actionLabel;
        }

        @Override
        public boolean equals(Object other) {
            if (!(other instanceof Field)) {
                return false;
            }
            Field field = (Field) other;
            return inputType == field.inputType
                    && imeOptions == field.imeOptions
                    && Objects.equals(actionLabel, field.actionLabel);
        }

        @Override
        public int hashCode() {
            return Objects.hash(inputType, imeOptions, actionLabel);
        }
    }

    /** A field's value: its text, selection and composing region, -1 where there is none. */
    private static final class State {
        static final State NONE = new State("", -1, -1, -1, -1);

        final String text;
        final int selectionStart;
        final int selectionEnd;
        final int composingStart;
        final int composingEnd;

        State(String text, int selectionStart, int selectionEnd, int composingStart, int composingEnd) {
            this.text = text;
            this.selectionStart = selectionStart;
            this.selectionEnd = selectionEnd;
            this.composingStart = composingStart;
            this.composingEnd = composingEnd;
        }

        static State of(SpannableStringBuilder editable) {
            return new State(
                    editable.toString(),
                    Selection.getSelectionStart(editable),
                    Selection.getSelectionEnd(editable),
                    BaseInputConnection.getComposingSpanStart(editable),
                    BaseInputConnection.getComposingSpanEnd(editable));
        }

        @Override
        public boolean equals(Object other) {
            if (!(other instanceof State)) {
                return false;
            }
            State state = (State) other;
            return text.equals(state.text)
                    && selectionStart == state.selectionStart
                    && selectionEnd == state.selectionEnd
                    && composingStart == state.composingStart
                    && composingEnd == state.composingEnd;
        }

        @Override
        public int hashCode() {
            return Objects.hash(text, selectionStart, selectionEnd, composingStart, composingEnd);
        }
    }
}
