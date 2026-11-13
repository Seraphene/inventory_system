# Tindahan Ledger 0.2 — Rust + native Android

Offline sari-sari store inventory and cash recording, intended for Samsung A32.
[Download APK](dist/tindahan-ledger.apk). Install over v0.1 without uninstalling: the package and signing key are unchanged, and old records migrate automatically.

## Pages
- **Summary:** available cash, historical sales, outside capital, purchases and deductions.
- **Items:** add/restock with total purchase cost and funding choice; edit details.
- **Sales:** find an item and record quantity sold.
- **Money Out:** deduct an amount with a reason.
- **Export:** inventory and transaction CSV using Android's save picker.

For a PHP 2,000 purchase with PHP 1,000 available, split funding uses PHP 1,000 store cash and records PHP 1,000 outside capital. Entirely external funding records PHP 2,000 outside capital and preserves the PHP 1,000 store cash. Purchases never reduce historical sales. Outside money is recorded inside the purchase. Stock corrections do not create purchases.

## Memory and storage
Rust handles accounting, validation, CSV and SQLite transactions. Java supplies native Android widgets and a single background worker. No WebView, JavaScript engine, network permission, accounts, analytics or background service.
The UI fetches at most 51 rows and displays 50 items per page. SQLite uses a 256 KiB page cache; CSV exports stream through 8 KiB buffers. The complete ledger is not loaded into RAM during normal use. The one-time migration must read the old JSON document.
These are design bounds, not measured phone RAM usage. Physical A32 RAM, launch timing and UI testing remain outstanding.

Records live in private SQLite app data. Clearing cache preserves them. Uninstalling or clearing app data deletes them. Export regularly. The old JSON table is retained after migration for recovery; new records use normalized tables. Money uses integer centavos and quantities are whole pieces.

## Build
Requires Rust stable, JDK 17, Android SDK platform 34/build-tools 34.0.0 and NDK r27c on Linux.
```sh
rustup target add aarch64-linux-android armv7-linux-androideabi
export JAVA_HOME=/path/to/jdk17
export ANDROID_HOME=/path/to/android-sdk
export ANDROID_NDK_HOME=/path/to/android-ndk-r27c
bash build-apk.sh
```
APK: `app/build/direct/tindahan-ledger.apk`. ARM64 and ARMv7 are included.
Android Studio's Gradle build also runs `build-native.sh`; Rust and NDK must be configured.

This is a locally signed testing APK. The ignored `.signing/testing.jks` key must be retained for in-place updates. Its development password is not suitable for a production signing key. Do not uninstall to update or records will be lost.

## Verification
`cargo test --locked --manifest-path native/Cargo.toml` covers funding choices, transaction rollback, stock validation, historical sales, migration/idempotency, failed migration preservation, file persistence, CSV quoting and bounded pagination.
`tests/jni/com/seraphene/inventory/MainActivity.java` smoke-tests the JNI entrypoint on a host JVM.
Both ARM libraries and Android UI compile. APK signature, ABIs and matching update certificate are checked with SDK tools. These tests do not replace testing on the actual phone.
