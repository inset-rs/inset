package rs.waft;

import android.content.Context;
import android.content.Intent;
import android.net.Uri;

/**
 * Waft's activity for a view the app asks for ({@code Platform::request_view}): a {@link
 * WaftActivity}, so it has text input, insets and Back, started in a task of its own, a card in
 * Recents, as a desktop makes a window for each request. Every request makes a new task ({@code
 * FLAG_ACTIVITY_NEW_DOCUMENT} with {@code FLAG_ACTIVITY_MULTIPLE_TASK}), started from the
 * application, as the manifest's {@code documentLaunchMode="intoExisting"} and {@code
 * launchMode="standard"} allow.
 *
 * <p>The intent it starts with is the request: its data is the view's initial data, under the
 * scheme {@link #SCHEME}, and its extras number the request and the process that made it. Android
 * keeps that intent as the task's, so a task it brings back by itself, a card tapped in Recents
 * after the process died, starts again with the same intent: the same data, and another process's
 * number, which tells the app's native code the view is one the platform gave, not one it asked
 * for. The invariant a later editor will break: the view's initial data is the data and nothing
 * else, since the data is all of a request that outlives the process.
 *
 * <p>A shortcut on the home screen may start the activity too ({@link #shortcutIntent}): the same
 * data, an action, and no request, so a view the platform gave.
 *
 * <p>The intent is never something to open: the native code reads it as the view's, not as an
 * intent to view, and hands on none that reaches the activity later, such as a shortcut's tapped
 * while its task runs, which only brings the task forward. waft-cli compiles this class
 * into every Android app.
 */
public final class WaftDocumentActivity extends WaftActivity {
    /** The scheme of the intent's data, whose scheme-specific part is the view's initial data. */
    static final String SCHEME = "waft-view";
    /** The extra holding the number of the process that made the request. */
    private static final String EXTRA_PROCESS = "rs.waft.view.PROCESS";
    /** The extra holding the request's number in that process. */
    private static final String EXTRA_REQUEST = "rs.waft.view.REQUEST";
    /** The request of an intent that names none. */
    private static final long NO_REQUEST = -1;

    /**
     * Starts the activity in a new task of its own from {@code context}, for the request numbered
     * {@code request} of the process numbered {@code process}, carrying {@code initialData}, which
     * may be null. Throws what {@code startActivity} throws, such as when the manifest declares no
     * such activity.
     */
    static void start(Context context, String initialData, long process, long request) {
        Intent intent = new Intent(context, WaftDocumentActivity.class);
        if (initialData != null) {
            intent.setData(Uri.fromParts(SCHEME, initialData, null));
        }
        intent.putExtra(EXTRA_PROCESS, process);
        intent.putExtra(EXTRA_REQUEST, request);
        intent.addFlags(
                Intent.FLAG_ACTIVITY_NEW_TASK
                        | Intent.FLAG_ACTIVITY_NEW_DOCUMENT
                        | Intent.FLAG_ACTIVITY_MULTIPLE_TASK);
        context.startActivity(intent);
    }

    /**
     * The intent of a shortcut that opens the view carrying {@code initialData}: the activity's
     * intent with that data, as a request's, and with an action, which a shortcut's intent needs,
     * but neither a request's number nor {@code FLAG_ACTIVITY_MULTIPLE_TASK}. So a tap finds the
     * task Android keeps with that data, which it finds by the activity and the data alone, or
     * starts one whose view the platform gives the app, a request of no process's.
     */
    static Intent shortcutIntent(Context context, String initialData) {
        Intent intent = new Intent(Intent.ACTION_MAIN);
        intent.setClass(context, WaftDocumentActivity.class);
        intent.setData(Uri.fromParts(SCHEME, initialData, null));
        return intent;
    }

    /** The view's initial data, as the activity's intent carries it; null for none. */
    String initialData() {
        return initialDataOf(getIntent());
    }

    /** The view's initial data {@code intent} carries, under {@link #SCHEME}; null for none. */
    static String initialDataOf(Intent intent) {
        Uri data = intent.getData();
        if (data == null || !SCHEME.equals(data.getScheme())) {
            return null;
        }
        return data.getSchemeSpecificPart();
    }

    /**
     * The number of the request the activity was started for, where the process numbered {@code
     * process} made it; {@link #NO_REQUEST} for a request of another process, or none.
     */
    long request(long process) {
        Intent intent = getIntent();
        if (intent.getLongExtra(EXTRA_PROCESS, process + 1) != process) {
            return NO_REQUEST;
        }
        return intent.getLongExtra(EXTRA_REQUEST, NO_REQUEST);
    }
}
