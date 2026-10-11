# Add project specific ProGuard rules here.
# You can control the set of applied configuration files using the
# proguardFiles setting in build.gradle.

# Keep WebRTC native classes
-keep class org.webrtc.** { *; }

# Keep Java-WebSocket classes
-keep class org.java_websocket.** { *; }

# Keep OkHttp classes
-keep class okhttp3.** { *; }
-dontwarn okhttp3.**

# Keep Gson if used
-keep class com.google.gson.** { *; }

# Keep PChome activities and services
-keep public class com.pchome.mobile.** { *; }

# Remove logging in release
-assumenosideeffects class android.util.Log {
    public static *** d(...);
    public static *** v(...);
    public static *** i(...);
}
