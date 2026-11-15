package rs.waft;

import android.content.Context;
import android.content.Intent;
import android.content.pm.ShortcutInfo;
import android.content.pm.ShortcutManager;
import android.graphics.Bitmap;
import android.graphics.drawable.AdaptiveIconDrawable;
import android.graphics.drawable.Icon;
import java.util.Arrays;
import java.util.List;

/**
 * The app's shortcuts, through the framework's {@link ShortcutManager}: a shortcut pinned on the
 * home screen, or dynamic, on the app's icon's menu, made from what the app's native code hands
 * over in plain values, and read back. A shortcut either opens a view in a task of its own, as
 * {@link WaftDocumentActivity#shortcutIntent} makes its intent, or brings Waft's activity forward
 * with an action the app reads ({@link #ACTION_TAP}). The calls the framework marks as for a
 * worker thread are made from a thread of the native code's own.
 *
 * <p>The invariant a later editor will break: a shortcut's icon is scaled down to the most the
 * system keeps before it is handed over, since it crosses into the system's process in one
 * transaction. waft-cli compiles this class into every Android app.
 */
final class WaftShortcuts {
    /** The action of a tap on a shortcut that brings Waft's activity forward. */
    static final String ACTION_TAP = "rs.waft.action.SHORTCUT";
    /** The extra holding the shortcut's id. */
    private static final String EXTRA_ID = "rs.waft.shortcut.ID";
    /** The extra holding the app's own action. */
    private static final String EXTRA_ACTION = "rs.waft.shortcut.ACTION";

    private WaftShortcuts() {}

    private static ShortcutManager manager(Context context) {
        return context.getSystemService(ShortcutManager.class);
    }

    /** Whether the default launcher pins a shortcut on request. */
    static boolean pins(Context context) {
        return manager(context).isRequestPinShortcutSupported();
    }

    /** The side, in pixels, of the visible part of a shortcut's icon as the launcher shows it. */
    static int iconMaxSize(Context context) {
        return manager(context).getIconMaxWidth();
    }

    static int maxPerActivity(Context context) {
        return manager(context).getMaxShortcutCountPerActivity();
    }

    static boolean isRateLimitingActive(Context context) {
        return manager(context).isRateLimitingActive();
    }

    /**
     * A shortcut: {@code id}, its labels, the long one and the disabled message null for none; its
     * icon {@code pixels}, non-premultiplied ARGB colours row by row, {@code width} by {@code
     * height}, null for none, adaptive or a plain bitmap; opening a view carrying {@code data}
     * where {@code opensView}, or else bringing Waft's activity forward with the action {@code
     * data}; its {@code rank} on the app icon's menu, negative for none.
     */
    static ShortcutInfo info(
            Context context,
            String id,
            String shortLabel,
            String longLabel,
            String disabledMessage,
            int[] pixels,
            int width,
            int height,
            boolean adaptive,
            boolean opensView,
            String data,
            int rank) {
        ShortcutInfo.Builder builder =
                new ShortcutInfo.Builder(context, id).setShortLabel(shortLabel);
        if (longLabel != null) {
            builder.setLongLabel(longLabel);
        }
        if (disabledMessage != null) {
            builder.setDisabledMessage(disabledMessage);
        }
        if (pixels != null) {
            builder.setIcon(icon(context, pixels, width, height, adaptive));
        }
        builder.setIntent(
                opensView
                        ? WaftDocumentActivity.shortcutIntent(context, data)
                        : tapIntent(context, id, data));
        if (rank >= 0) {
            builder.setRank(rank);
        }
        return builder.build();
    }

    /**
     * Asks the launcher to pin {@code shortcut}, which asks the user; answers whether the launcher
     * took the request. Throws what {@code requestPinShortcut} throws: where the app has nothing in
     * front, or a disabled shortcut has the same id.
     */
    static boolean requestPin(Context context, ShortcutInfo shortcut) {
        return manager(context).requestPinShortcut(shortcut, null);
    }

    /** Updates {@code shortcuts}, pinned or dynamic; false where Android limits the app now. */
    static boolean update(Context context, ShortcutInfo[] shortcuts) {
        return manager(context).updateShortcuts(Arrays.asList(shortcuts));
    }

    static boolean setDynamic(Context context, ShortcutInfo[] shortcuts) {
        return manager(context).setDynamicShortcuts(Arrays.asList(shortcuts));
    }

    static boolean addDynamic(Context context, ShortcutInfo[] shortcuts) {
        return manager(context).addDynamicShortcuts(Arrays.asList(shortcuts));
    }

    static void pushDynamic(Context context, ShortcutInfo shortcut) {
        manager(context).pushDynamicShortcut(shortcut);
    }

    static void removeDynamic(Context context, String[] ids) {
        manager(context).removeDynamicShortcuts(Arrays.asList(ids));
    }

    static void removeAllDynamic(Context context) {
        manager(context).removeAllDynamicShortcuts();
    }

    static void reportUsed(Context context, String id) {
        manager(context).reportShortcutUsed(id);
    }

    static void enable(Context context, String[] ids) {
        manager(context).enableShortcuts(Arrays.asList(ids));
    }

    /** Disables {@code ids}, which say {@code message} when tapped, or the system's, where null. */
    static void disable(Context context, String[] ids, String message) {
        List<String> list = Arrays.asList(ids);
        if (message == null) {
            manager(context).disableShortcuts(list);
        } else {
            manager(context).disableShortcuts(list, message);
        }
    }

    /** The app's shortcuts {@code flags} match ({@code ShortcutManager.FLAG_MATCH_*}). */
    static ShortcutInfo[] shortcuts(Context context, int flags) {
        return manager(context).getShortcuts(flags).toArray(new ShortcutInfo[0]);
    }

    /** {@code shortcut}'s long label, as text; null for none. */
    static String longLabel(ShortcutInfo shortcut) {
        CharSequence label = shortcut.getLongLabel();
        return label == null ? null : label.toString();
    }

    /** {@code shortcut}'s short label, as text; null for none. */
    static String shortLabel(ShortcutInfo shortcut) {
        CharSequence label = shortcut.getShortLabel();
        return label == null ? null : label.toString();
    }

    /** The view's initial data a shortcut opening a view carries; null for any other. */
    static String viewData(ShortcutInfo shortcut) {
        Intent intent = shortcut.getIntent();
        return intent == null ? null : WaftDocumentActivity.initialDataOf(intent);
    }

    /** The app's action a shortcut bringing Waft's activity forward carries; null for any other. */
    static String tapAction(ShortcutInfo shortcut) {
        Intent intent = shortcut.getIntent();
        return intent == null || !ACTION_TAP.equals(intent.getAction())
                ? null
                : intent.getStringExtra(EXTRA_ACTION);
    }

    /** The tap {@code intent} is, its shortcut's id and the app's action; null for no tap. */
    static String[] tapOf(Intent intent) {
        if (!ACTION_TAP.equals(intent.getAction())) {
            return null;
        }
        String id = intent.getStringExtra(EXTRA_ID);
        String action = intent.getStringExtra(EXTRA_ACTION);
        return id == null || action == null ? null : new String[] {id, action};
    }

    /** The intent of a shortcut that brings Waft's activity forward with {@code action}. */
    private static Intent tapIntent(Context context, String id, String action) {
        Intent intent = new Intent(ACTION_TAP);
        intent.setClass(context, WaftActivity.class);
        intent.putExtra(EXTRA_ID, id);
        intent.putExtra(EXTRA_ACTION, action);
        return intent;
    }

    /**
     * The icon of {@code pixels}, scaled down to what the system keeps: the visible part's most
     * for a plain bitmap, and for an adaptive one that and the band around it the launcher shows
     * only as the icon moves.
     */
    private static Icon icon(Context context, int[] pixels, int width, int height, boolean adaptive) {
        Bitmap bitmap = Bitmap.createBitmap(pixels, width, height, Bitmap.Config.ARGB_8888);
        int visible = iconMaxSize(context);
        int largest =
                adaptive
                        ? Math.round(
                                visible * (1 + 2 * AdaptiveIconDrawable.getExtraInsetFraction()))
                        : visible;
        int side = Math.max(width, height);
        if (largest > 0 && side > largest) {
            float scale = (float) largest / side;
            bitmap =
                    Bitmap.createScaledBitmap(
                            bitmap,
                            Math.max(1, Math.round(width * scale)),
                            Math.max(1, Math.round(height * scale)),
                            true);
        }
        return adaptive ? Icon.createWithAdaptiveBitmap(bitmap) : Icon.createWithBitmap(bitmap);
    }
}
