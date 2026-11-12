package rs.waft;

import android.app.NativeActivity;
import android.content.Intent;
import android.os.Bundle;
import android.view.ViewGroup;

/**
 * Waft's activity: the platform's {@link NativeActivity}, which loads the app's native library
 * and runs it, with five things added, which NativeActivity keeps to itself or cannot give.
 *
 * <ul>
 *   <li>An activity the app starts for a result, such as the system's document picker, answers
 *       through {@link #onActivityResult}; this hands each result on to the app's native code.
 *   <li>An intent that reaches the running activity, such as a file or a link to view, or a tap on
 *       one of the app's notifications, arrives through {@link #onNewIntent}; this makes it the
 *       activity's intent and hands it on. The intent the activity started with the native code
 *       reads itself.
 *   <li>The user's answer to a permission the app asked for, such as posting notifications,
 *       arrives through {@link #onRequestPermissionsResult}; this hands it on.
 *   <li>The system's keyboards edit text only through a view's input connection, which
 *       NativeActivity's content view does not give; this adds {@link WaftTextInput} beside it,
 *       which does, and which the native code reaches through {@link #textInput}.
 *   <li>What the system's bars and the keyboard cover of the window arrives as the window's
 *       insets, where NativeActivity tells only the rectangle its content was laid out in; this
 *       has {@link WaftWindowInsets} hear the window's decor view and hand the insets on, which
 *       the native code reaches through {@link #windowInsets}.
 * </ul>
 *
 * <p>The native methods are registered by the app's native code when it starts
 * ({@code RegisterNatives}), since NativeActivity loads the app's library itself, where the
 * runtime would not look for it by name. waft-cli compiles this class into every Android app.
 */
public class WaftActivity extends NativeActivity {
    private WaftTextInput textInput;
    private WaftWindowInsets windowInsets;

    @Override
    protected void onCreate(Bundle savedInstanceState) {
        // Both made before NativeActivity starts the app's native code, which asks for them at
        // once.
        textInput = new WaftTextInput(this);
        windowInsets = new WaftWindowInsets();
        super.onCreate(savedInstanceState);
        windowInsets.hear(getWindow().getDecorView());
        addContentView(
                textInput,
                new ViewGroup.LayoutParams(
                        ViewGroup.LayoutParams.MATCH_PARENT, ViewGroup.LayoutParams.MATCH_PARENT));
    }

    /** The view the system's keyboards edit text through, for the app's native code. */
    WaftTextInput textInput() {
        return textInput;
    }

    /** What tells the window's insets, for the app's native code. */
    WaftWindowInsets windowInsets() {
        return windowInsets;
    }

    /**
     * Does what Android does with a Back press nothing handled, on the main thread, for the app's
     * native code, which asks from its own thread once the app has left the press unhandled: the
     * system's own {@code onBackPressed}, which since Android 12 moves a launcher's root activity
     * to the background and finishes any other.
     */
    void backLeftUnhandled() {
        runOnUiThread(
                new Runnable() {
                    @Override
                    public void run() {
                        WaftActivity.super.onBackPressed();
                    }
                });
    }

    @Override
    protected void onActivityResult(int requestCode, int resultCode, Intent data) {
        super.onActivityResult(requestCode, resultCode, data);
        onResult(requestCode, resultCode, data);
    }

    @Override
    protected void onNewIntent(Intent intent) {
        super.onNewIntent(intent);
        setIntent(intent);
        try {
            onIntent(intent);
        } catch (UnsatisfiedLinkError notYetRegistered) {
            // The native code has not started yet; it reads the activity's intent when it does.
        }
    }

    @Override
    public void onRequestPermissionsResult(
            int requestCode, String[] permissions, int[] grantResults) {
        super.onRequestPermissionsResult(requestCode, permissions, grantResults);
        onPermissionsResult(requestCode, grantResults);
    }

    /** Hands a result to the app's native code, which registered it. */
    private static native void onResult(int requestCode, int resultCode, Intent data);

    /** Hands an intent that reached this activity to the app's native code, which registered it. */
    private native void onIntent(Intent intent);

    /** Hands the answer to a permission asked for to the app's native code, which registered it. */
    private static native void onPermissionsResult(int requestCode, int[] grantResults);
}
