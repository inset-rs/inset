package com.example.adder;

import java.io.ByteArrayOutputStream;
import java.io.IOException;
import java.io.InputStream;

/** A plain JAR's class, and a Java resource beside it. */
public final class Adder {
    private Adder() {}

    public static int add(int a, int b) {
        return a + b;
    }

    /** The text of the Java resource `answer.txt`, which the APK carries at its root. */
    public static String answer() throws IOException {
        try (InputStream in = Adder.class.getResourceAsStream("/com/example/adder/answer.txt")) {
            if (in == null) {
                return "no answer.txt";
            }
            ByteArrayOutputStream out = new ByteArrayOutputStream();
            byte[] buffer = new byte[256];
            for (int n; (n = in.read(buffer)) > 0; ) {
                out.write(buffer, 0, n);
            }
            return out.toString("UTF-8").trim();
        }
    }
}
