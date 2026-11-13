#!/usr/bin/env bash
set -euo pipefail
# Direct SDK build with Rust/SQLite and native Android widgets.
: "${ANDROID_HOME:?Set ANDROID_HOME to the Android SDK directory}"
: "${JAVA_HOME:?Set JAVA_HOME to a JDK}"
: "${ANDROID_NDK_HOME:?Set ANDROID_NDK_HOME to Android NDK r27c}"
bash build-native.sh
bt="$ANDROID_HOME/build-tools/34.0.0"
platform="$ANDROID_HOME/platforms/android-34/android.jar"
mkdir -p app/build/direct/classes app/build/direct/dex app/build/direct/gen
stage=app/build/direct
classes=$(mktemp -d "$stage/classes.XXXXXX")
# aapt needs a package attribute (Gradle normally supplies the namespace).
sed 's/<manifest xmlns:/<manifest package="com.seraphene.inventory" xmlns:/' app/src/main/AndroidManifest.xml > "$stage/AndroidManifest.xml"
"$bt/aapt" package -f -M "$stage/AndroidManifest.xml" -S app/src/main/res -I "$platform" -F "$stage/resources.apk" -J "$stage/gen" --min-sdk-version 23 --target-sdk-version 34 --version-code 2 --version-name 0.2.0
"$JAVA_HOME/bin/javac" --release 8 -classpath "$platform" -d "$classes" app/src/main/java/com/seraphene/inventory/MainActivity.java
"$JAVA_HOME/bin/jar" cf "$stage/classes.jar" -C "$classes" .
"$bt/d8" --lib "$platform" --min-api 23 --output "$stage/dex" "$stage/classes.jar"
cp "$stage/resources.apk" "$stage/unsigned.apk"
(cd "$stage/dex" && zip -q ../unsigned.apk classes*.dex)
mkdir -p "$stage/lib"
cp -R app/build/rust-jni/arm64-v8a app/build/rust-jni/armeabi-v7a "$stage/lib/"
(cd "$stage" && zip -qr unsigned.apk lib)
"$bt/zipalign" -f 4 "$stage/unsigned.apk" "$stage/aligned.apk"
mkdir -p .signing
if [ ! -f .signing/testing.jks ]; then
  "$JAVA_HOME/bin/keytool" -genkeypair -keystore .signing/testing.jks -storepass android -keypass android -alias testing -keyalg RSA -keysize 2048 -validity 10000 -dname 'CN=Tindahan local testing'
fi
"$bt/apksigner" sign --ks .signing/testing.jks --ks-key-alias testing --ks-pass pass:android --out "$stage/tindahan-ledger.apk" "$stage/aligned.apk"
"$bt/apksigner" verify --verbose "$stage/tindahan-ledger.apk"
