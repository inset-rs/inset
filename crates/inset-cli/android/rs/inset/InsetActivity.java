package rs.inset;

import android.app.NativeActivity;
import android.content.Intent;

/**
 * Inset's activity: the platform's {@link NativeActivity}, which loads the app's native library
 * and runs it, with one thing added. An activity the app starts for a result, such as the
 * system's document picker, answers through {@link #onActivityResult}, which NativeActivity keeps
 * to itself; this hands each result on to the app's native code.
 *
 * <p>The native method is registered by the app's native code when it starts
 * ({@code RegisterNatives}), since NativeActivity loads the app's library itself, where the
 * runtime would not look for it by name. inset-cli compiles this class into every Android app.
 */
public class InsetActivity extends NativeActivity {
    @Override
    protected void onActivityResult(int requestCode, int resultCode, Intent data) {
        super.onActivityResult(requestCode, resultCode, data);
        onResult(requestCode, resultCode, data);
    }

    /** Hands a result to the app's native code, which registered it. */
    private static native void onResult(int requestCode, int resultCode, Intent data);
}
