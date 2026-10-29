package rs.inset;

/**
 * Inset's activity for a view of its own: each instance is a document task, Android's window,
 * with its own card in Recents. A view the app asks for starts one, its intent carrying the view's
 * initial data as its data; Android finds a task again by that data, and starts the activity again
 * with it when it brings the task back after the app's process was gone.
 *
 * <p>It is {@link InsetActivity}, declared apart in the manifest only because a document task needs
 * the {@code standard} launch mode, where the launcher's activity is {@code singleTask}, so that
 * files, links and notification taps reach the one task the app started in. inset-cli compiles this
 * class into every Android app and declares it, not exported.
 */
public class InsetDocumentActivity extends InsetActivity {}
