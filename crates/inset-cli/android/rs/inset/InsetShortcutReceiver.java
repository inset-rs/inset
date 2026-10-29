package rs.inset;

import android.content.BroadcastReceiver;
import android.content.Context;
import android.content.Intent;

/**
 * Hears that the launcher pinned one of the app's shortcuts: the answer the launcher sends to the
 * request {@code ShortcutManager.requestPinShortcut} made, after the user confirmed it. It hands the
 * request's number to the app's native code, which registered {@link #onPinned}. Android tells no
 * app that the user declined, so nothing arrives then.
 *
 * <p>inset-cli compiles this class into every Android app and declares it, not exported: only the
 * launcher's answer to the app's own request reaches it.
 */
public final class InsetShortcutReceiver extends BroadcastReceiver {
    /** The extra numbering the request the answer is for. */
    public static final String EXTRA_REQUEST = "rs.inset.shortcut.REQUEST";

    @Override
    public void onReceive(Context context, Intent intent) {
        try {
            onPinned(intent.getIntExtra(EXTRA_REQUEST, -1));
        } catch (UnsatisfiedLinkError notLoaded) {
            // The answer came to a process the system started for it, where the app's native code
            // is not loaded: nobody is waiting for it.
        }
    }

    /** Hands the answer to the app's native code, which registered it. */
    private static native void onPinned(int request);
}
