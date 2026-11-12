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
 * <p>The native method is registered by the app's native code when it starts, which may be after
 * the first insets were applied: it then asks for them again ({@link #tellAgain}). waft-cli
 * compiles this class into every Android app.
 */
final class WaftWindowInsets implements View.OnApplyWindowInsetsListener {
    private final Handler mainThread = new Handler(Looper.getMainLooper());
    /** The window's decor view, once the activity has one; the main thread's alone. */
    private View decorView;

    /** Hears the insets Android applies to the window {@code decorView} is the root of. */
    void hear(View decorView) {
        this.decorView = decorView;
        decorView.setOnApplyWindowInsetsListener(this);
    }

    /**
     * Has Android apply the window's insets once more, on the main thread, for the app's native
     * code, which asks from its own thread once it can hear them. The main thread comes to it
     * after the activity's {@code onCreate}, which is where the decor view is given.
     */
    void tellAgain() {
        mainThread.post(() -> decorView.requestApplyInsets());
    }

    @Override
    public WindowInsets onApplyWindowInsets(View view, WindowInsets insets) {
        tell(insets);
        // The decor view goes on to do its own with them, as it does with no listener set.
        return view.onApplyWindowInsets(insets);
    }

    private void tell(WindowInsets insets) {
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
        try {
            onWindowInsets(
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
        } catch (UnsatisfiedLinkError notYetRegistered) {
            // The native code has not started yet; it asks for the insets again when it does.
        }
    }

    /**
     * Hands the window's insets to the app's native code, which registered it: the view padding,
     * the view insets and then the system gesture insets, each side in pixels.
     */
    private native void onWindowInsets(
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
