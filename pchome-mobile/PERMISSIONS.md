# PChome Android Permissions Guide

## Required Permissions

### AndroidManifest.xml

The following permissions must be declared in `AndroidManifest.xml`:

```xml
<manifest ... >
    <uses-permission android:name="android.permission.INTERNET" />
    <uses-permission android:name="android.permission.ACCESS_NETWORK_STATE" />
    <uses-permission android:name="android.permission.WRITE_EXTERNAL_STORAGE" />
    <uses-permission android:name="android.permission.READ_EXTERNAL_STORAGE" />
    
    <!-- MediaProjection for screen capture -->
    <uses-permission android:name="android.permission.MEDIA_PROJECTION" />
    
    <!-- AccessibilityService for input injection -->
    <uses-permission android:name="android.permission.BIND_ACCESSIBILITY_SERVICE" />
    
    <!-- Foreground service permissions (if applicable) -->
    <uses-permission android:name="android.permission.FOREGROUND_SERVICE" />
    <uses-permission android:name="android.permission.FOREGROUND_SERVICE_MEDIA_PROJECTION" />
</manifest>
```

### Firebase/Google Play Services (Optional)
If using cloud relay features, add:
```xml
<uses-permission android:name="android.permission.ACCESS_FINE_LOCATION" />
<uses-permission android:name="android.permission.ACCESS_COARSE_LOCATION" />
```

## AccessibilityService Configuration

### Service Declaration
```xml
<service
    android:name=".service.AndroidControlService"
    android:permission="android.permission.BIND_ACCESSIBILITY_SERVICE"
    android:exported="false">
    <intent-filter>
        <action android:name="android.accessibilityservice.AccessibilityService" />
    </intent-filter>
    
    <!-- Accessibility configuration -->
    <meta-data
        android:name="android.accessibilityservice"
        android:resource="@xml/accessibility_config" />
    
    <meta-data
        android:name="android.permission.FOREGROUND_SERVICE"
        android:resource="@xml/foreground_service_config" />
</service>
```

### accessibility_config.xml
Create `res/xml/accessibility_config.xml`:
```xml
<accessibility-service
    android:description="@string/accessibility_description"
    android:permission="android.permission.BIND_ACCESSIBILITY_SERVICE"
    android:accessibilityFlags="flagDefault"
    android:accessibilityEventTypes="all"
    android:canRetrieveWindowContent="true"
    android:notificationTimeout="100"
    android:packageNames="com.pchome.mobile"
    android:settingsActivity="com.pchome.mobile.ui.PinActivity" />
```

### foreground_service_config.xml
Create `res/xml/foreground_service_config.xml`:
```xml
<foreground-service
    android:minDelayMillis="1000"
    android:stopWithTask="true" />
```

## MediaProjection Configuration

### ScreenCaptureService.java Setup
```java
// Required in onCreate()
private val projectionManager = getSystemService(ProjectionManager::class.java)

// Launch projection selection
val intent = projectionManager.createScreenCaptureIntent()
startActivityForResult(intent, REQUEST_PROJECTION)
```

### onActivityResult Handling
```kotlin
override fun onActivityResult(requestCode: Int, resultCode: Int, data: Intent?) {
    super.onActivityResult(requestCode, resultCode, data)
    if (requestCode == REQUEST_PROJECTION && resultCode == RESULT_OK) {
        projection = data?.getExtras()?.getInt("android.media.projection.extra.SESSION_ID")
        // Initialize MediaCodec encoder with projection
    }
}
```

## Permissions at Runtime

### Requesting Accessibility Permission
```kotlin
// Show system dialog to user
val intent = AccessibilityServiceIntent { 
    feedbackType = AccessibilityServiceIntent.FEEDBACK_ALL
}
sendAccessibilityServiceIntent(intent)
```

### Requesting MediaProjection Permission
```kotlin
// Already handled via createScreenCaptureIntent()
// User must accept system dialog
```

## Permission Testing

### Verify Permissions Are Granted
```bash
# Check installed permissions
adb shell pm list permissions -d -f com.pchome.mobile

# Verify Accessibility is enabled
adb shell settings put secure enabled_accessibility_services com.pchome.mobile/.service.AndroidControlService

# Test screen capture
adb shell screenrecord /sdcard/test.mp4 --time-limit 5
```

## Security Implications

### Minimum Required Permissions
1. `INTERNET` - Signal server WebSocket connectivity
2. `ACCESS_NETWORK_STATE` - Network status monitoring
3. `MEDIA_PROJECTION` - Screen capture (Android 10+)
4. `BIND_ACCESSIBILITY_SERVICE` - Input injection (all Android versions)

### Optional Permissions
- `WRITE_EXTERNAL_STORAGE/READ_EXTERNAL_STORAGE` - For screen recording/playback
- `FOREGROUND_SERVICE` - Long-running capture sessions
- `ACCESS_FINE_COARSE_LOCATION` - Google Play Services features

### Permission Denial Handling
- Gracefully degrade functionality if permissions denied
- Show explanatory UI requesting necessary permissions
- Do not attempt to bypass system permission restrictions