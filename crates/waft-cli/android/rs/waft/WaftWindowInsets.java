package rs.waft;

import android.graphics.Insets;
import android.os.Handler;
import android.os.Looper;
import android.view.DisplayCutout;
import android.view.View;
import android.view.WindowInsets;

/**
 * Waft's window insets: what the system covers of the activity's window, told to the app's native
 * code each time Android applies the window's insets. NativeActivity tells its native code only
 * the rectangle its content view was laid out in, which does not say the system's bars from the
 * keyboard, so {@link WaftActivity} has this hear the window's decor view, the first view the
 * insets are applied to and so the one that sees them whole, before any view takes its share.
 *
 * <p>The app draws on the window's whole surface, so what it is told is measured from the
 * window's edges, in pixels: the system's bars and the display's cutout, which Flutter calls the
 * view padding, the keyboard, which Flutter calls the view insets, and the zones where the system
 * takes a swipe for its own navigation, which Flutter calls the system gesture insets. This is the
 * part of Flutter's Android {@code FlutterView.onApplyWindowInsets} for Android 11 and later, which
 * is every release Waft runs on.
 *
 * <p>The app's native code is one for the process, whichever activities it has, and each
 * activity's window is one of its views: the native code gives this the number of its view
 * ({@link #attach}), and every telling carries it, as the Java side of Flutter's embedding carries
 * its shell's id into each native call ({@code FlutterJNI.nativeShellHolderId}). Until then
 * nothing is told: the native code registers its method before it attaches a listener, and has
 * Android apply the insets again as it does, since the first were applied before. waft-cli
 * compiles this class into every Android app.
 */
final class WaftWindowInsets implements View.OnApplyWindowInsetsListener {
    /** No view: the native code has not attached this yet. */
    private static final long NO_VIEW = -1;

    private final Handler mainThread = new Handler(Looper.getMainLooper());
    /** The window's decor view, once the activity has one; the main thread's alone. */
    private View decorView;
    /** The number the native code gave the view this window is; the main thread's alone. */
    private long view = NO_VIEW;

    /** Hears the insets Android applies to the window {@code decorView} is the root of. */
    void hear(View decorView) {
        this.decorView = decorView;
        decorView.setOnApplyWindowInsetsListener(this);
    }

    /**
     * Tells the window's insets as those of the view numbered {@code view} from now on, and has
     * Android apply them once more, on the main thread, for the app's native code, which asks from
     * its own thread once it can hear them. The main thread comes to it after the activity's
     * {@code onCreate}, which is where the decor view is given.
     */
    void attach(long view) {
        mainThread.post(
                () -> {
                    this.view = view;
                    decorView.requestApplyInsets();
                });
    }

    @Override
    public WindowInsets onApplyWindowInsets(View view, WindowInsets insets) {
        tell(insets);
        // The decor view goes on to do its own with them, as it does with no listener set.
        return view.onApplyWindowInsets(insets);
    }

    private void tell(WindowInsets insets) {
        if (view == NO_VIEW) {
            return;
        }
        Insets uiInsets = insets.getInsets(WindowInsets.Type.systemBars());
        int viewPaddingLeft = uiInsets.left;
        int viewPaddingTop = uiInsets.top;
        int viewPaddingRight = uiInsets.right;
        int viewPaddingBottom = uiInsets.bottom;

        // Take the max of the display cutout insets and existing padding to merge them
        DisplayCutout cutout = insets.getDisplayCutout();
        if (cutout != null) {
            Insets waterfallInsets = cutout.getWaterfallInsets();
            viewPaddingLeft =
                    Math.max(
                            Math.max(viewPaddingLeft, waterfallInsets.left),
                            cutout.getSafeInsetLeft());
            viewPaddingTop =
                    Math.max(
                            Math.max(viewPaddingTop, waterfallInsets.top),
                            cutout.getSafeInsetTop());
            viewPaddingRight =
                    Math.max(
                            Math.max(viewPaddingRight, waterfallInsets.right),
                            cutout.getSafeInsetRight());
            viewPaddingBottom =
                    Math.max(
                            Math.max(viewPaddingBottom, waterfallInsets.bottom),
                            cutout.getSafeInsetBottom());
        }

        Insets imeInsets = insets.getInsets(WindowInsets.Type.ime());
        Insets systemGestureInsets = insets.getInsets(WindowInsets.Type.systemGestures());
        onWindowInsets(
                view,
                viewPaddingLeft,
                viewPaddingTop,
                viewPaddingRight,
                viewPaddingBottom,
                imeInsets.left,
                imeInsets.top,
                imeInsets.right,
                imeInsets.bottom, // Typically, only bottom is non-zero
                systemGestureInsets.left,
                systemGestureInsets.top,
                systemGestureInsets.right,
                systemGestureInsets.bottom);
    }

    /**
     * Hands the insets of the window the view numbered {@code view} is to the app's native code,
     * which registered it: the view padding, the view insets and then the system gesture insets,
     * each side in pixels.
     */
    private native void onWindowInsets(
            long view,
            int viewPaddingLeft,
            int viewPaddingTop,
            int viewPaddingRight,
            int viewPaddingBottom,
            int viewInsetLeft,
            int viewInsetTop,
            int viewInsetRight,
            int viewInsetBottom,
            int systemGestureInsetLeft,
            int systemGestureInsetTop,
            int systemGestureInsetRight,
            int systemGestureInsetBottom);
}
