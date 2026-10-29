package rs.inset;

import android.app.AlarmManager;
import android.app.Notification;
import android.app.NotificationChannel;
import android.app.NotificationManager;
import android.app.PendingIntent;
import android.app.RemoteInput;
import android.content.BroadcastReceiver;
import android.content.Context;
import android.content.Intent;
import android.graphics.Bitmap;
import android.graphics.drawable.Icon;
import android.net.Uri;
import android.os.Build;
import android.os.Bundle;

/**
 * Inset's notifications, as the app's native code and the system's alarms reach them. A
 * notification arrives as a {@link Bundle} the native code fills, under the keys below: its id,
 * title, body, image, actions and whether it is silent. It is built here and shown under its id as
 * the tag, so a later one under the same id replaces it, on one of two channels of the app's: one
 * that sounds and one that does not. A tap on it, on one of its buttons, or a reply typed into it
 * starts Inset's activity with an intent of {@link #ACTIVATED}, which names the notification and
 * the action and carries the reply, and which the native code reads.
 *
 * <p>A notification for later travels whole in its alarm's intent, as the app may not run when the
 * alarm goes off: this receiver hears the alarm and shows the notification. The alarm is exact
 * where the system lets the app set exact alarms, and inexact otherwise. The system forgets alarms
 * when the device restarts.
 *
 * <p>inset-cli compiles this class into every Android app and declares it in the manifest, not
 * exported: only the app's own alarms reach it.
 */
public final class InsetNotificationReceiver extends BroadcastReceiver {
    /** The action of the intent a tap, a button or a reply starts Inset's activity with. */
    public static final String ACTIVATED = "rs.inset.action.NOTIFICATION_ACTIVATED";
    /** The extra of an activation naming the notification's id. */
    public static final String EXTRA_ID = "rs.inset.notification.ID";
    /** The extra of an activation naming the action's id; none for a tap on the notification. */
    public static final String EXTRA_ACTION = "rs.inset.notification.ACTION";
    /** The key of a reply among a {@link RemoteInput}'s results. */
    public static final String REPLY = "rs.inset.notification.REPLY";

    // The bundle's keys, which the native code fills.
    private static final String ID = "id";
    private static final String TITLE = "title";
    private static final String BODY = "body";
    private static final String SILENT = "silent";
    /** The image's pixels as non-premultiplied ARGB colours, row by row. */
    private static final String PIXELS = "pixels";
    private static final String WIDTH = "width";
    private static final String HEIGHT = "height";
    /** The actions' ids, labels and reply placeholders, one each an action; no placeholder for a button. */
    private static final String ACTION_IDS = "actionIds";
    private static final String ACTION_LABELS = "actionLabels";
    private static final String PLACEHOLDERS = "placeholders";
    /** The image, made from the pixels and fitted to the system's large icon. */
    private static final String IMAGE = "image";

    /** The action of an alarm's intent, which carries its notification. */
    private static final String DUE = "rs.inset.action.NOTIFICATION_DUE";
    private static final String EXTRA_NOTIFICATION = "rs.inset.notification.NOTIFICATION";
    /** The scheme of the data that tells each notification's intents from another's. */
    private static final String SCHEME = "inset-notification";
    private static final String ACTIVITY = "rs.inset.InsetActivity";
    /** The number every notification is shown under, beside its id as the tag. */
    private static final int NUMBER = 1;
    private static final String CHANNEL = "rs.inset.notifications";
    private static final String SILENT_CHANNEL = "rs.inset.notifications.silent";

    @Override
    public void onReceive(Context context, Intent intent) {
        Bundle notification = intent.getBundleExtra(EXTRA_NOTIFICATION);
        if (DUE.equals(intent.getAction()) && notification != null) {
            notify(context, notification);
        }
    }

    /** Shows the notification now, in place of the one shown or waiting under its id. */
    public static void show(Context context, Bundle notification) {
        cancelAlarm(context, notification.getString(ID));
        notify(context, fitted(context, notification));
    }

    /**
     * Shows the notification at {@code at}, in milliseconds since the epoch, in place of the one
     * waiting under its id.
     */
    public static void schedule(Context context, Bundle notification, long at) {
        PendingIntent due =
                alarm(context, notification.getString(ID), fitted(context, notification),
                        PendingIntent.FLAG_UPDATE_CURRENT);
        AlarmManager alarms = context.getSystemService(AlarmManager.class);
        if (Build.VERSION.SDK_INT < 31 || alarms.canScheduleExactAlarms()) {
            alarms.setExactAndAllowWhileIdle(AlarmManager.RTC_WAKEUP, at, due);
        } else {
            alarms.setAndAllowWhileIdle(AlarmManager.RTC_WAKEUP, at, due);
        }
    }

    /** Takes away the notification shown or waiting under {@code id}. */
    public static void remove(Context context, String id) {
        dismiss(context, id);
        cancelAlarm(context, id);
    }

    /** Takes away the notification shown under {@code id}, as one of its actions answers it. */
    public static void dismiss(Context context, String id) {
        context.getSystemService(NotificationManager.class).cancel(id, NUMBER);
    }

    private static void notify(Context context, Bundle notification) {
        context.getSystemService(NotificationManager.class)
                .notify(notification.getString(ID), NUMBER, built(context, notification));
    }

    private static Notification built(Context context, Bundle notification) {
        String id = notification.getString(ID);
        Notification.Builder builder =
                new Notification.Builder(context, channel(context, notification.getBoolean(SILENT)))
                        .setSmallIcon(smallIcon(context))
                        .setContentTitle(notification.getString(TITLE))
                        .setAutoCancel(true)
                        .setContentIntent(activation(context, id, null, false));
        String body = notification.getString(BODY);
        if (body != null) {
            builder.setContentText(body).setStyle(new Notification.BigTextStyle().bigText(body));
        }
        Bitmap image = image(notification);
        if (image != null) {
            builder.setLargeIcon(image);
        }
        String[] ids = notification.getStringArray(ACTION_IDS);
        String[] labels = notification.getStringArray(ACTION_LABELS);
        String[] placeholders = notification.getStringArray(PLACEHOLDERS);
        for (int index = 0; ids != null && index < ids.length; index++) {
            builder.addAction(action(context, id, ids[index], labels[index], placeholders[index]));
        }
        return builder.build();
    }

    /** The notification's image, through the untyped getter before Android 13 brought the typed. */
    @SuppressWarnings("deprecation")
    private static Bitmap image(Bundle notification) {
        return Build.VERSION.SDK_INT >= 33
                ? notification.getParcelable(IMAGE, Bitmap.class)
                : notification.getParcelable(IMAGE);
    }

    /** A button, or a field to reply in where it has a placeholder. */
    private static Notification.Action action(
            Context context, String id, String action, String label, String placeholder) {
        boolean reply = placeholder != null;
        Notification.Action.Builder builder =
                new Notification.Action.Builder(
                        (Icon) null, label, activation(context, id, action, reply));
        if (reply) {
            builder.addRemoteInput(new RemoteInput.Builder(REPLY).setLabel(placeholder).build());
        }
        return builder.build();
    }

    /**
     * Starts Inset's activity with an activation of {@code action} on the notification {@code id};
     * its data tells it from every other notification's and action's. Only a reply's may be
     * changed, as the system fills the reply in.
     */
    private static PendingIntent activation(
            Context context, String id, String action, boolean reply) {
        Intent intent =
                new Intent(ACTIVATED)
                        .setClassName(context, ACTIVITY)
                        .setData(Uri.fromParts(SCHEME, id, action))
                        .putExtra(EXTRA_ID, id)
                        .addFlags(Intent.FLAG_ACTIVITY_NEW_TASK);
        if (action != null) {
            intent.putExtra(EXTRA_ACTION, action);
        }
        int mutability =
                !reply
                        ? PendingIntent.FLAG_IMMUTABLE
                        : Build.VERSION.SDK_INT >= 31 ? PendingIntent.FLAG_MUTABLE : 0;
        return PendingIntent.getActivity(
                context, 0, intent, mutability | PendingIntent.FLAG_UPDATE_CURRENT);
    }

    /**
     * The alarm that shows the notification {@code id}; {@code null} with {@code
     * FLAG_NO_CREATE} where none is set. Its intent's data tells it from another notification's.
     */
    private static PendingIntent alarm(Context context, String id, Bundle notification, int flags) {
        Intent intent =
                new Intent(DUE)
                        .setClass(context, InsetNotificationReceiver.class)
                        .setData(Uri.fromParts(SCHEME, id, null));
        if (notification != null) {
            intent.putExtra(EXTRA_NOTIFICATION, notification);
        }
        return PendingIntent.getBroadcast(
                context, 0, intent, flags | PendingIntent.FLAG_IMMUTABLE);
    }

    private static void cancelAlarm(Context context, String id) {
        PendingIntent due = alarm(context, id, null, PendingIntent.FLAG_NO_CREATE);
        if (due == null) {
            return;
        }
        context.getSystemService(AlarmManager.class).cancel(due);
        due.cancel();
    }

    /**
     * The channel the notification is shown on, made the first time: both pop up, as the other
     * systems' notifications do, and the silent one makes no sound and does not vibrate. The user
     * may change either in the app's settings.
     */
    private static String channel(Context context, boolean silent) {
        NotificationManager manager = context.getSystemService(NotificationManager.class);
        if (!silent) {
            manager.createNotificationChannel(
                    new NotificationChannel(
                            CHANNEL, "Notifications", NotificationManager.IMPORTANCE_HIGH));
            return CHANNEL;
        }
        NotificationChannel quiet =
                new NotificationChannel(
                        SILENT_CHANNEL, "Silent notifications", NotificationManager.IMPORTANCE_HIGH);
        quiet.setSound(null, null);
        quiet.enableVibration(false);
        manager.createNotificationChannel(quiet);
        return SILENT_CHANNEL;
    }

    /** The app's icon, or the system's default app icon for an app without one. */
    private static int smallIcon(Context context) {
        int icon = context.getApplicationInfo().icon;
        return icon != 0 ? icon : android.R.drawable.sym_def_app_icon;
    }

    /** The notification with its pixels made the image it shows, no larger than a large icon. */
    private static Bundle fitted(Context context, Bundle notification) {
        int[] pixels = notification.getIntArray(PIXELS);
        if (pixels == null) {
            return notification;
        }
        Bundle fitted = new Bundle(notification);
        fitted.remove(PIXELS);
        fitted.putParcelable(
                IMAGE,
                largeIcon(context, pixels, notification.getInt(WIDTH), notification.getInt(HEIGHT)));
        return fitted;
    }

    private static Bitmap largeIcon(Context context, int[] pixels, int width, int height) {
        Bitmap image = Bitmap.createBitmap(pixels, width, height, Bitmap.Config.ARGB_8888);
        int largest =
                context.getResources()
                        .getDimensionPixelSize(android.R.dimen.notification_large_icon_width);
        int side = Math.max(width, height);
        if (side <= largest) {
            return image;
        }
        float scale = (float) largest / side;
        return Bitmap.createScaledBitmap(
                image,
                Math.max(1, Math.round(width * scale)),
                Math.max(1, Math.round(height * scale)),
                true);
    }
}
