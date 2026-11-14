#!/bin/sh
# Builds the example's two Android libraries from their sources here, laid out as Android's Gradle
# plugin lays them out: adder.jar, a class and a Java resource beside it, and greeter.aar, a
# manifest, classes compiled against an R class that classes.jar leaves out, resources, and a
# native library for 64-bit ARM. Needs the SDK (ANDROID_HOME), its NDK, and a JDK (JAVA_HOME).
set -e
here=$(cd "$(dirname "$0")" && pwd)
sdk=${ANDROID_HOME:-$HOME/Library/Android/sdk}
jdk=${JAVA_HOME:-/Applications/Android Studio.app/Contents/jbr/Contents/Home}/bin
android_jar=$(ls -d "$sdk"/platforms/android-*/android.jar | sort -V | tail -n 1)
ndk=$(ls -d "$sdk"/ndk/* | sort -V | tail -n 1)
clang=$(ls "$ndk"/toolchains/llvm/prebuilt/*/bin/aarch64-linux-android31-clang | head -n 1)
work=$(mktemp -d)
trap 'rm -r "$work"' EXIT

mkdir -p "$work/adder/com/example/adder"
"$jdk/javac" --release 11 -d "$work/adder" "$here/adder/src/com/example/adder/Adder.java"
cp "$here/adder/answer.txt" "$work/adder/com/example/adder/"
rm -f "$here/adder.jar"
(cd "$work/adder" && "$jdk/jar" --create --file "$here/adder.jar" .)

mkdir -p "$work/classes" "$work/aar/jni/arm64-v8a"
"$jdk/javac" --release 11 -Xlint:-options -classpath "$android_jar:$here/adder.jar" -d "$work/classes" "$here"/greeter/src/com/example/greeter/*.java
rm "$work"/classes/com/example/greeter/R.class "$work"/classes/com/example/greeter/R\$*.class
(cd "$work/classes" && "$jdk/jar" --create --file "$work/aar/classes.jar" .)
cp "$here/greeter/AndroidManifest.xml" "$work/aar/"
cp -R "$here/greeter/res" "$work/aar/res"
printf 'int string greeter_flavour 0x0\nint string greeter_greeting 0x0\nint xml network_security_config 0x0\n' > "$work/aar/R.txt"
printf -- '-keep class com.example.greeter.** { *; }\n' > "$work/aar/proguard.txt"
"$clang" -shared -o "$work/aar/jni/arm64-v8a/libgreeter_native.so" "$here/greeter/native/greeter_native.c"
rm -f "$here/greeter.aar"
(cd "$work/aar" && zip -qr "$here/greeter.aar" .)
