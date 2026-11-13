# Tindahan Ledger — Android
Offline inventory and cash records for a small sari-sari store. Android 6+; intended for the Samsung A32.

## Install
Download [tindahan-ledger.apk](dist/tindahan-ledger.apk), open it on the phone, and allow installation from that download source when Android asks. This is a locally signed testing build (v0.1.0), not a Play Store release.

## Included
- Add/restock items with quantity, actual total purchase cost and selling price.
- Payment options: store cash plus outside-money shortage, fully outside money, or store cash only.
- Sales reduce stock and increase available cash. Historical sales are unchanged by purchases.
- Money taken out requires an amount and reason.
- Edit item name, stock correction, cost and selling price without rewriting historical transactions.
- Inventory CSV, transaction CSV, and full JSON data export using Android's Save document picker.
- Whole-piece quantities; money is calculated in integer centavos.

Cash = sales + outside capital - stock purchases - withdrawals.
Example: with PHP 1,000 available and a PHP 2,000 purchase, split funding records PHP 1,000 outside capital and leaves zero cash. Entirely external funding records PHP 2,000 outside capital and leaves PHP 1,000 cash.

## Storage and implementation
Java Android shell with bundled HTML/JavaScript and Android WebView. This version is not Rust. SQLite stores the ledger as an atomic JSON document in the private app database directory. Cache clearing does not delete it. Uninstalling or clearing app data does. Export regularly. JSON export is a data archive; in-app restore is not included.
No internet permission, accounts, ads, analytics or background services.

## Build
Use JDK 17, Android platform 34 and build-tools 34.0.0:
```sh
export JAVA_HOME=/path/to/jdk17
export ANDROID_HOME=/path/to/android-sdk
bash build-apk.sh
```
Output: `app/build/direct/tindahan-ledger.apk`.
The script creates a local testing key under ignored `.signing/`. Keep this key for updates that preserve app data. A different key cannot update an existing installation. The testing key uses a public development password and is not suitable for a production release.
An Android Gradle project is also included for Android Studio.

## Verification
`node test-ledger.cjs` checks split/external funding, insufficient cash, sales, stock, history and deductions. APK signature and manifest were verified with Android SDK tools. Browser UI tests use a mocked Android storage/export bridge. Physical A32 runtime and real document-picker testing remain outstanding.
