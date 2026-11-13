#!/usr/bin/env bash
set -euo pipefail
: "${ANDROID_NDK_HOME:?Set ANDROID_NDK_HOME to Android NDK r27c}"
ndkbin="$ANDROID_NDK_HOME/toolchains/llvm/prebuilt/linux-x86_64/bin"
export CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER="$ndkbin/aarch64-linux-android23-clang"
export CARGO_TARGET_ARMV7_LINUX_ANDROIDEABI_LINKER="$ndkbin/armv7a-linux-androideabi23-clang"
export CC_aarch64_linux_android="$CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER"
export CC_armv7_linux_androideabi="$CARGO_TARGET_ARMV7_LINUX_ANDROIDEABI_LINKER"
export AR_aarch64_linux_android="$ndkbin/llvm-ar"
export AR_armv7_linux_androideabi="$ndkbin/llvm-ar"
export CARGO_TARGET_AARCH64_LINUX_ANDROID_RUSTFLAGS='-C link-arg=-Wl,-z,max-page-size=16384'
cargo build --locked --release --manifest-path native/Cargo.toml --target aarch64-linux-android --target armv7-linux-androideabi
mkdir -p app/build/rust-jni/arm64-v8a app/build/rust-jni/armeabi-v7a
cp native/target/aarch64-linux-android/release/libtindahan.so app/build/rust-jni/arm64-v8a/
cp native/target/armv7-linux-androideabi/release/libtindahan.so app/build/rust-jni/armeabi-v7a/
