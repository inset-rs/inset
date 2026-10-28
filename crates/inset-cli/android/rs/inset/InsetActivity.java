package rs.inset;

import android.app.NativeActivity;
import android.content.Intent;

/**
 * Inset's activity: the platform's {@link NativeActivity}, which loads the app's native library
 * and runs it, with two things added, which NativeActivity keeps to itself.
 *
 * <ul>
 *   <li>An activity the app starts for a result, such as the system's document picker, answers
 *       through {@link #onActivityResult}; this hands each result on to the app's native code.
 *   <li>An intent that reaches the running activity, such as a file or a link to view, arrives
 *       through {@link #onNewIntent}; this makes it the activity's intent and hands it on. The
 *       intent the activity started with the native code reads itself.
 * </ul>
 *
 * <p>The native methods are registered by the app's native code when it starts
 * ({@code RegisterNatives}), since NativeActivity loads the app's library itself, where the
 * runtime would not look for it by name. inset-cli compiles this class into every Android app.
 */
public class InsetActivity extends NativeActivity {
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

    /** Hands a result to the app's native code, which registered it. */
    private static native void onResult(int requestCode, int resultCode, Intent data);

    /** Hands an intent to the app's native code, which registered it. */
    private static native void onIntent(Intent intent);
}
