package rs.inset;

import android.content.ContentProvider;
import android.content.ContentValues;
import android.content.Context;
import android.content.pm.ProviderInfo;
import android.database.Cursor;
import android.database.MatrixCursor;
import android.net.Uri;
import android.os.ParcelFileDescriptor;
import android.provider.OpenableColumns;
import android.webkit.MimeTypeMap;
import java.io.File;
import java.io.FileNotFoundException;
import java.util.ArrayList;
import java.util.List;
import java.util.Locale;

/**
 * Inset's file provider: the app's own files, shared with the app that opens one. Another app
 * cannot read a file by its path in this app's storage, so the app's native code hands it a
 * {@code content://} URI of this provider instead, whose path is the file's, and grants it reading
 * that one URI with the intent that opens it.
 *
 * <p>The provider is not exported: no other app reaches it but through a URI it was granted, and
 * it shares a file to read only. It answers what a viewer asks of a document: the file, its name
 * and size, and its type, from its extension. inset-cli compiles this class into every Android app
 * and declares it in the manifest, at the authority {@code <package>.inset.files}.
 */
public final class InsetFileProvider extends ContentProvider {
    private static final String[] COLUMNS = {OpenableColumns.DISPLAY_NAME, OpenableColumns.SIZE};

    @Override
    public void attachInfo(Context context, ProviderInfo info) {
        super.attachInfo(context, info);
        if (info.exported) {
            throw new SecurityException("Inset's file provider must not be exported");
        }
        if (!info.grantUriPermissions) {
            throw new SecurityException("Inset's file provider must grant URI permissions");
        }
    }

    @Override
    public boolean onCreate() {
        return true;
    }

    @Override
    public ParcelFileDescriptor openFile(Uri uri, String mode) throws FileNotFoundException {
        if (!"r".equals(mode)) {
            throw new SecurityException("Inset's file provider shares files to read only");
        }
        return ParcelFileDescriptor.open(file(uri), ParcelFileDescriptor.MODE_READ_ONLY);
    }

    @Override
    public Cursor query(
            Uri uri, String[] projection, String selection, String[] arguments, String order) {
        File file = file(uri);
        List<String> columns = new ArrayList<>();
        List<Object> values = new ArrayList<>();
        for (String column : projection != null ? projection : COLUMNS) {
            if (OpenableColumns.DISPLAY_NAME.equals(column)) {
                columns.add(column);
                values.add(file.getName());
            } else if (OpenableColumns.SIZE.equals(column)) {
                columns.add(column);
                values.add(file.length());
            }
        }
        MatrixCursor cursor = new MatrixCursor(columns.toArray(new String[0]), 1);
        cursor.addRow(values.toArray());
        return cursor;
    }

    @Override
    public String getType(Uri uri) {
        String name = file(uri).getName();
        int dot = name.lastIndexOf('.');
        if (dot >= 0) {
            String extension = name.substring(dot + 1).toLowerCase(Locale.ROOT);
            String type = MimeTypeMap.getSingleton().getMimeTypeFromExtension(extension);
            if (type != null) {
                return type;
            }
        }
        return "application/octet-stream";
    }

    @Override
    public Uri insert(Uri uri, ContentValues values) {
        throw new UnsupportedOperationException("Inset's file provider shares files to read only");
    }

    @Override
    public int update(Uri uri, ContentValues values, String selection, String[] arguments) {
        throw new UnsupportedOperationException("Inset's file provider shares files to read only");
    }

    @Override
    public int delete(Uri uri, String selection, String[] arguments) {
        throw new UnsupportedOperationException("Inset's file provider shares files to read only");
    }

    /** The file a URI of this provider names: its path is the file's. */
    private static File file(Uri uri) {
        return new File(uri.getPath());
    }
}
