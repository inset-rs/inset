package rs.inset;

import android.app.NativeActivity;
import android.content.Intent;

/**
 * Inset's activity: the platform's {@link NativeActivity}, which loads the app's native library
 * and runs it, with three things added, which NativeActivity keeps to itself.
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
