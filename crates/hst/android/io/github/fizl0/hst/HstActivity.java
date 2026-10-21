package io.github.fizl0.hst;

import android.app.NativeActivity;
import android.app.PendingIntent;
import android.content.Intent;
import android.content.pm.PackageInstaller;
import android.net.wifi.WifiManager;
import android.os.Build;
import android.os.Bundle;
import android.os.ParcelFileDescriptor;
import android.view.View;
import android.view.WindowInsets;
import android.view.WindowInsetsController;
import android.view.WindowManager;
import java.io.FileInputStream;
import java.io.InputStream;
import java.io.OutputStream;

/** The game's NativeActivity (`tools/apk.sh`), plus what the native side can't do on its own (M99): the system's file
 *  picker for the disc image, and a restart once the disc is imported. Called from `crates/hst-base/src/android.rs`. */
public class HstActivity extends NativeActivity {
    private static final int PICK = 1;
    /** The picked document's file descriptor (the native side owns it), -1 when the picker was cancelled, -2 when
     *  there's nothing new. */
    private volatile int picked = -2;
    /** B455: Wi-Fi drops multicast an app hasn't asked for, so mDNS (the LAN's lobbies, `net.rs` `watch_lan`) heard
     *  nothing: held while the game is in front. */
    private WifiManager.MulticastLock lan;
    /** M140: the update install's `PackageInstaller` status, -99 until the installer answers. */
    private volatile int installStatus = -99;
    /** M140: the action of the installer's answer, which comes back to this activity (singleTop: `onNewIntent`). */
    private static final String INSTALLED = "io.github.fizl0.hst.INSTALLED";

    /** M106: the window reaches under the camera cutout on every Android, as it must from Android 15 on; the game keeps
     *  its picture inside the safe area itself (`android.rs` `safe_area`, `widescreen.rs`). */
    @Override
    protected void onCreate(Bundle state) {
        if (Build.VERSION.SDK_INT >= 28) {
            WindowManager.LayoutParams a = getWindow().getAttributes();
            a.layoutInDisplayCutoutMode = Build.VERSION.SDK_INT >= 30
                ? WindowManager.LayoutParams.LAYOUT_IN_DISPLAY_CUTOUT_MODE_ALWAYS
                : WindowManager.LayoutParams.LAYOUT_IN_DISPLAY_CUTOUT_MODE_SHORT_EDGES;
            getWindow().setAttributes(a);
        }
        super.onCreate(state);
        immersive();
        try {
            WifiManager w = (WifiManager) getApplicationContext().getSystemService(WIFI_SERVICE);
            if (w != null) {
                lan = w.createMulticastLock("hst-lan");
                lan.setReferenceCounted(false);
            }
        } catch (Exception e) {
            lan = null;
        }
    }

    @Override
    protected void onResume() {
        super.onResume();
        try {
            if (lan != null) lan.acquire();
        } catch (Exception e) {
            // no CHANGE_WIFI_MULTICAST_STATE: the LAN list stays empty, the internet's still works
        }
    }

    /** B623: the game fills the whole screen, under the status and navigation bars too (before Android 15 the window
     *  stopped above the navigation bar, while its insets still counted it), and hides them: a swipe from the edge shows
     *  them for a moment over the game. The safe area the HUD keeps to is then only the camera cutout (`android.rs`). */
    private void immersive() {
        if (Build.VERSION.SDK_INT >= 30) {
            getWindow().setDecorFitsSystemWindows(false);
            WindowInsetsController c = getWindow().getDecorView().getWindowInsetsController();
            if (c != null) {
                c.setSystemBarsBehavior(WindowInsetsController.BEHAVIOR_SHOW_TRANSIENT_BARS_BY_SWIPE);
                c.hide(WindowInsets.Type.systemBars());
            }
        } else {
            getWindow().getDecorView().setSystemUiVisibility(View.SYSTEM_UI_FLAG_IMMERSIVE_STICKY
                | View.SYSTEM_UI_FLAG_HIDE_NAVIGATION | View.SYSTEM_UI_FLAG_FULLSCREEN
                | View.SYSTEM_UI_FLAG_LAYOUT_HIDE_NAVIGATION | View.SYSTEM_UI_FLAG_LAYOUT_FULLSCREEN);
        }
    }

    /** The system brings the bars back after a dialog, the picker or a trip to another app: hidden again on return. */
    @Override
    public void onWindowFocusChanged(boolean focus) {
        super.onWindowFocusChanged(focus);
        if (focus) immersive();
    }

    @Override
    protected void onPause() {
        if (lan != null && lan.isHeld()) lan.release();
        super.onPause();
    }

    /** Opens the system's document picker (any file: a disc image has no MIME type of its own). */
    public void pick() {
        runOnUiThread(new Runnable() {
            public void run() {
                Intent i = new Intent(Intent.ACTION_OPEN_DOCUMENT);
                i.addCategory(Intent.CATEGORY_OPENABLE);
                i.setType("*/*");
                startActivityForResult(i, PICK);
            }
        });
    }

    /** The picker's answer since the last call (see `picked`), then -2 again. */
    public int taken() {
        int fd = picked;
        if (fd != -2) picked = -2;
        return fd;
    }

    @Override
    protected void onActivityResult(int request, int result, Intent data) {
        if (request != PICK) {
            super.onActivityResult(request, result, data);
            return;
        }
        int fd = -1;
        if (result == RESULT_OK && data != null && data.getData() != null) {
            try {
                ParcelFileDescriptor p = getContentResolver().openFileDescriptor(data.getData(), "r");
                if (p != null) fd = p.detachFd();
            } catch (Exception e) {
                fd = -1;
            }
        }
        picked = fd;
    }

    /** M140: whether Google Play installed the app (then only Play may update it). */
    public boolean fromStore() {
        try {
            String by = Build.VERSION.SDK_INT >= 30
                ? getPackageManager().getInstallSourceInfo(getPackageName()).getInstallingPackageName()
                : installerBefore30();
            return "com.android.vending".equals(by);
        } catch (Exception e) {
            return false;
        }
    }

    /** `fromStore` before Android 11, whose only way to ask is the call 11 deprecated. */
    @SuppressWarnings("deprecation")
    private String installerBefore30() {
        return getPackageManager().getInstallerPackageName(getPackageName());
    }

    /** M140: the update .apk at `path` (downloaded and checked by `update.rs`) to the system's installer, as a
     *  `PackageInstaller` session; its answer arrives in `onNewIntent`, read by `installed`. Same signing key, so it
     *  installs over this app and keeps its files. */
    public void install(final String path) {
        installStatus = -99;
        new Thread(new Runnable() {
            public void run() {
                try {
                    PackageInstaller pi = getPackageManager().getPackageInstaller();
                    PackageInstaller.SessionParams p = new PackageInstaller.SessionParams(PackageInstaller.SessionParams.MODE_FULL_INSTALL);
                    p.setAppPackageName(getPackageName());
                    int id = pi.createSession(p);
                    PackageInstaller.Session s = pi.openSession(id);
                    try (InputStream in = new FileInputStream(path); OutputStream out = s.openWrite("base.apk", 0, -1)) {
                        byte[] b = new byte[1 << 16];
                        for (int n; (n = in.read(b)) > 0; ) out.write(b, 0, n);
                        s.fsync(out);
                    }
                    Intent i = new Intent(HstActivity.this, HstActivity.class).setAction(INSTALLED);
                    int flags = PendingIntent.FLAG_UPDATE_CURRENT | (Build.VERSION.SDK_INT >= 31 ? PendingIntent.FLAG_MUTABLE : 0);
                    s.commit(PendingIntent.getActivity(HstActivity.this, 0, i, flags).getIntentSender());
                    s.close();
                } catch (Exception e) {
                    installStatus = PackageInstaller.STATUS_FAILURE;
                }
            }
        }).start();
    }

    /** M140: the install's status since `install` (`PackageInstaller.STATUS_*`), -99 while there's none. */
    public int installed() {
        return installStatus;
    }

    @Override
    protected void onNewIntent(Intent i) {
        super.onNewIntent(i);
        if (!INSTALLED.equals(i.getAction())) return;
        int status = i.getIntExtra(PackageInstaller.EXTRA_STATUS, PackageInstaller.STATUS_FAILURE);
        if (status == PackageInstaller.STATUS_PENDING_USER_ACTION) {
            // the confirm screen (and, the first time, Android's "allow installs from this app")
            Intent confirm = Build.VERSION.SDK_INT >= 33 ? i.getParcelableExtra(Intent.EXTRA_INTENT, Intent.class) : intentBefore33(i);
            if (confirm != null) {
                startActivity(confirm.addFlags(Intent.FLAG_ACTIVITY_NEW_TASK));
                return;
            }
            status = PackageInstaller.STATUS_FAILURE;
        }
        installStatus = status;
    }

    /** `onNewIntent`'s confirm screen before Android 13, whose only way to read it is the call 13 deprecated. */
    @SuppressWarnings("deprecation")
    private static Intent intentBefore33(Intent i) {
        return i.getParcelableExtra(Intent.EXTRA_INTENT);
    }

    /** A fresh process for the game: the native side's window can only be made once per process. */
    public void restart() {
        startActivity(new Intent(this, Restart.class).putExtra("pid", android.os.Process.myPid()));
    }
}
