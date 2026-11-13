package com.seraphene.inventory;
/** Host JVM smoke test of the same exported JNI entrypoint used on Android. */
public class MainActivity {
    private static native String nativeCall(String path, String request);
    public static void main(String[] args) throws Exception {
        System.load(args[0]);
        java.nio.file.Path dir=java.nio.file.Files.createTempDirectory("tindahan-jni-");
        String path=dir.resolve("ledger.db").toString();
        String r=nativeCall(path,"{\"op\":\"purchase\",\"name\":\"Rice\",\"qty\":20,\"amount\":200000,\"price\":12000,\"mode\":\"outside\"}");
        if(!r.contains("\"ok\":true"))throw new AssertionError(r);
        r=nativeCall(path,"{\"op\":\"list\",\"search\":\"\",\"offset\":0}");
        if(!r.contains("\"outside\":200000")||!r.contains("\"qty\":20"))throw new AssertionError(r);
        r=nativeCall(path,"{\"op\":\"out\",\"amount\":1,\"reason\":\"Overdraw\"}");
        if(!r.contains("\"ok\":false"))throw new AssertionError(r);
        java.nio.file.Files.delete(dir.resolve("ledger.db"));java.nio.file.Files.delete(dir);
        System.out.println("PASS: JNI calls, persistent Rust SQLite storage, errors returned to Java");
    }
}
