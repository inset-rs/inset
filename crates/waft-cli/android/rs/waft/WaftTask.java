package rs.waft;

import android.app.Activity;
import android.app.ActivityManager;
import android.content.Context;
import android.graphics.Bitmap;
import android.graphics.drawable.Icon;
import android.os.Build;
import java.lang.reflect.Method;
import java.util.List;

/**
 * An activity's task, which is the window of the view the activity is: its card in Recents, which
 * shows the task's description, bringing it forward, and removing it while its activity is
 * destroyed. The app's native code calls these, the description on the main thread, where an
 * activity is touched.
 *
 * <p>Recents draws the description's icon beside the card and reads its label out to
 * accessibility, but draws no label of its own. The icon is no resource of the package, so it
 * goes through the description's {@code Builder.setIcon(Icon)} from Android 17, which may refuse
 * it there behind a system flag, and otherwise through the constructor that takes a {@code
 * Bitmap}, which Android deprecated but which is the only way to give a bitmap before 17. waft-cli
 * compiles this class into every Android app.
 */
final class WaftTask {
    /** Android 17, whose task description's builder takes an icon as an {@code Icon}. */
    private static final int ICON_BY_BUILDER = 37;

    private WaftTask() {}

    /**
     * Describes {@code activity}'s task: {@code label}, which may be null; the icon {@code
     * pixels}, non-premultiplied ARGB colours row by row, {@code width} by {@code height}, scaled
     * down to the launcher's icon size, or none where null; and {@code color}, opaque or 0 for the
     * theme's.
     */
    static void describe(
            Activity activity, String label, int[] pixels, int width, int height, int color) {
        Bitmap icon = pixels == null ? null : icon(activity, pixels, width, height);
        ActivityManager.TaskDescription description =
                Build.VERSION.SDK_INT >= ICON_BY_BUILDER ? built(label, icon, color) : null;
        if (description == null) {
            description = new ActivityManager.TaskDescription(label, icon, color);
        }
        activity.setTaskDescription(description);
    }

    /**
     * Brings the app's task numbered {@code taskId} forward, which Android allows while the app
     * has a window in sight; answers whether the app has such a task.
     */
    static boolean moveToFront(Context context, int taskId) {
        ActivityManager.AppTask task = task(context, taskId);
        if (task == null) {
            return false;
        }
        task.moveToFront();
        return true;
    }

    /**
     * Finishes the app's task numbered {@code taskId} and removes its card, for a task whose
     * activity Android destroyed and has not made again; answers whether the app has such a task.
     */
    static boolean remove(Context context, int taskId) {
        ActivityManager.AppTask task = task(context, taskId);
        if (task == null) {
            return false;
        }
        task.finishAndRemoveTask();
        return true;
    }

    /** The numbers of the app's tasks that Android keeps, in Recents or running. */
    static int[] taskIds(Context context) {
        ActivityManager manager = context.getSystemService(ActivityManager.class);
        List<ActivityManager.AppTask> tasks = manager.getAppTasks();
        int[] ids = new int[tasks.size()];
        for (int i = 0; i < ids.length; i++) {
            ids[i] = tasks.get(i).getTaskInfo().taskId;
        }
        return ids;
    }

    /** The app's task numbered {@code taskId}; null where Android keeps none so numbered. */
    private static ActivityManager.AppTask task(Context context, int taskId) {
        ActivityManager manager = context.getSystemService(ActivityManager.class);
        for (ActivityManager.AppTask task : manager.getAppTasks()) {
            if (task.getTaskInfo().taskId == taskId) {
                return task;
            }
        }
        return null;
    }

    /**
     * The description through the builder, the icon through {@code setIcon(Icon)}, which the
     * SDK this is compiled against may not name; null where the builder refuses it.
     */
    private static ActivityManager.TaskDescription built(String label, Bitmap icon, int color) {
        ActivityManager.TaskDescription.Builder builder =
                new ActivityManager.TaskDescription.Builder().setLabel(label);
        if (color != 0) {
            builder.setPrimaryColor(color);
        }
        if (icon != null) {
            try {
                Method setIcon =
                        ActivityManager.TaskDescription.Builder.class.getMethod(
                                "setIcon", Icon.class);
                setIcon.invoke(builder, Icon.createWithBitmap(icon));
            } catch (ReflectiveOperationException | RuntimeException refused) {
                return null;
            }
        }
        return builder.build();
    }

    /** The icon as a bitmap no larger than the launcher's icons. */
    private static Bitmap icon(Context context, int[] pixels, int width, int height) {
        Bitmap icon = Bitmap.createBitmap(pixels, width, height, Bitmap.Config.ARGB_8888);
        int largest = context.getSystemService(ActivityManager.class).getLauncherLargeIconSize();
        int side = Math.max(width, height);
        if (side <= largest) {
            return icon;
        }
        float scale = (float) largest / side;
        return Bitmap.createScaledBitmap(
                icon,
                Math.max(1, Math.round(width * scale)),
                Math.max(1, Math.round(height * scale)),
                true);
    }
}
