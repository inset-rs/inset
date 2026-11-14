package com.example.greeter;

import android.content.Context;
import android.content.pm.ApplicationInfo;
import android.content.pm.PackageManager;
import android.content.res.XmlResourceParser;
import com.example.adder.Adder;
import org.xmlpull.v1.XmlPullParser;

/** An Android library's class: reads its own resources through its R class, the application's
 *  meta-data its manifest adds, loads its native library and calls a class of a plain JAR. */
public final class Greeter {
    private Greeter() {}

    public static String greet(Context context) throws Exception {
        System.loadLibrary("greeter_native");
        StringBuilder answer = new StringBuilder();
        answer.append(context.getString(R.string.greeter_greeting));
        ApplicationInfo info = context.getPackageManager()
                .getApplicationInfo(context.getPackageName(), PackageManager.GET_META_DATA);
        answer.append(" | meta-data=").append(info.metaData.getString("com.example.greeter.flavour"));
        answer.append(" | config domain=").append(firstDomain(context));
        answer.append(" | native loaded");
        answer.append(" | 2+3=").append(Adder.add(2, 3));
        answer.append(" | java resource=").append(Adder.answer());
        return answer.toString();
    }

    private static String firstDomain(Context context) throws Exception {
        try (XmlResourceParser parser = context.getResources().getXml(R.xml.network_security_config)) {
            for (int event = parser.getEventType(); event != XmlPullParser.END_DOCUMENT; event = parser.next()) {
                if (event == XmlPullParser.START_TAG && parser.getName().equals("domain")) {
                    return parser.nextText();
                }
            }
        }
        return "none";
    }
}
