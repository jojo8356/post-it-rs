//! System event detection for boot/wake triggers
//! Platform-specific implementations for Windows, Linux, macOS

use std::sync::mpsc;
use std::thread;
use anyhow::Result;
use tracing::{info, warn};
#[cfg(any(target_os = "windows", target_os = "macos"))]
use tracing::debug;
#[cfg(target_os = "windows")]
use tracing::error;

/// Events that can trigger post-it display
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SystemEvent {
    Boot,
    Wake,
}

/// System listener trait
pub trait SystemListener: Send + Sync {
    fn start(&mut self, tx: mpsc::Sender<SystemEvent>) -> Result<()>;
    fn stop(&mut self) -> Result<()>;
    fn name(&self) -> &'static str;
}

/// Cross-platform system listener factory
pub fn create_listener() -> Box<dyn SystemListener> {
    #[cfg(target_os = "windows")]
    return Box::new(WindowsListener::new());
    
    #[cfg(target_os = "linux")]
    return Box::new(LinuxListener::new());
    
    #[cfg(target_os = "macos")]
    return Box::new(MacOSListener::new());
    
    #[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
    return Box::new(StubListener::new());
}

/// Stub listener for unsupported platforms
pub struct StubListener;

impl StubListener {
    pub fn new() -> Self { Self }
}

impl SystemListener for StubListener {
    fn start(&mut self, _tx: mpsc::Sender<SystemEvent>) -> Result<()> {
        warn!("Stub listener: no boot/wake detection on this platform");
        Ok(())
    }
    fn stop(&mut self) -> Result<()> { Ok(()) }
    fn name(&self) -> &'static str { "stub" }
}

// ============================================================================
// Windows Implementation
// ============================================================================

#[cfg(target_os = "windows")]
pub mod windows {
    use super::*;
    use windows::Win32::Foundation::*;
    use windows::Win32::System::Power::*;
    use windows::Win32::UI::WindowsAndMessaging::*;
    use windows::Win32::System::LibraryLoader::GetModuleHandleW;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;
    use std::ptr;

    pub struct WindowsListener {
        running: Arc<AtomicBool>,
        thread_handle: Option<thread::JoinHandle<()>>,
    }

    impl WindowsListener {
        pub fn new() -> Self {
            Self { running: Arc::new(AtomicBool::new(false)), thread_handle: None }
        }
    }

    impl SystemListener for WindowsListener {
        fn start(&mut self, tx: mpsc::Sender<SystemEvent>) -> Result<()> {
            if self.running.swap(true, Ordering::SeqCst) {
                return Ok(());
            }

            let running = self.running.clone();
            let handle = thread::spawn(move || {
                unsafe {
                    // Create a message-only window to receive power broadcast messages
                    let class_name = windows::core::w!("PostItPowerWindow");
                    let wc = WNDCLASSEXW {
                        cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
                        style: CS_HREDRAW | CS_VREDRAW,
                        lpfnWndProc: Some(wnd_proc),
                        cbClsExtra: 0,
                        cbWndExtra: 0,
                        hInstance: GetModuleHandleW(None).unwrap_or(HINSTANCE(0)),
                        hIcon: HICON(0),
                        hCursor: HCURSOR(0),
                        hbrBackground: HBRUSH(0),
                        lpszMenuName: None,
                        lpszClassName: class_name,
                        hIconSm: HICON(0),
                    };
                    
                    let atom = RegisterClassExW(&wc);
                    if atom == 0 {
                        error!("Failed to register window class for power events");
                        return;
                    }

                    let hwnd = CreateWindowExW(
                        WINDOW_EX_STYLE(0),
                        class_name,
                        windows::core::w!("PostItPowerListener"),
                        WS_OVERLAPPEDWINDOW,
                        0, 0, 0, 0,
                        HWND_MESSAGE, // Message-only window
                        None,
                        GetModuleHandleW(None).unwrap_or(HINSTANCE(0)),
                        Some(Box::into_raw(Box::new(tx)) as *mut _),
                    );

                    if hwnd.0 == 0 {
                        error!("Failed to create message window for power events");
                        return;
                    }

                    info!("Windows power event listener started");

                    // Message loop
                    let mut msg = MSG::default();
                    while running.load(Ordering::SeqCst) {
                        if PeekMessageW(&mut msg, HWND(0), 0, 0, PM_REMOVE).into() {
                            let _ = TranslateMessage(&msg);
                            DispatchMessageW(&msg);
                        } else {
                            thread::sleep(std::time::Duration::from_millis(100));
                        }
                    }

                    DestroyWindow(hwnd);
                    UnregisterClassW(class_name, GetModuleHandleW(None).unwrap_or(HINSTANCE(0)));
                }
            });

            self.thread_handle = Some(handle);
            Ok(())
        }

        fn stop(&mut self) -> Result<()> {
            self.running.store(false, Ordering::SeqCst);
            if let Some(handle) = self.thread_handle.take() {
                let _ = handle.join();
            }
            Ok(())
        }

        fn name(&self) -> &'static str { "windows" }
    }

    unsafe extern "system" fn wnd_proc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
        if msg == WM_POWERBROADCAST {
            if let Some(tx_ptr) = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut mpsc::Sender<SystemEvent> {
                let tx = &*tx_ptr;
                match wparam.0 as u32 {
                    PBT_APMRESUMEAUTOMATIC => {
                        info!("Windows: System resumed from sleep (automatic)");
                        let _ = tx.send(SystemEvent::Wake);
                    }
                    PBT_APMRESUMESUSPEND => {
                        info!("Windows: System resumed from sleep (suspend)");
                        let _ = tx.send(SystemEvent::Wake);
                    }
                    PBT_APMSUSPEND => {
                        debug!("Windows: System suspending");
                    }
                    _ => {}
                }
            }
            return LRESULT(1);
        }
        DefWindowProcW(hwnd, msg, wparam, lparam)
    }
}

#[cfg(target_os = "windows")]
use windows::Win32::UI::WindowsAndMessaging::GWLP_USERDATA;

// Re-export Windows listener
#[cfg(target_os = "windows")]
pub use windows::WindowsListener;

// ============================================================================
// Linux Implementation
// ============================================================================

#[cfg(target_os = "linux")]
pub mod linux {
    use super::*;
    use std::process::Command;
    use std::time::Duration;
    use tokio::time::interval;

    pub struct LinuxListener {
        use_systemd: bool,
        handle: Option<thread::JoinHandle<()>>,
    }

    impl LinuxListener {
        pub fn new() -> Self {
            // Check if systemd is available
            let use_systemd = Command::new("systemctl")
                .arg("--version")
                .output()
                .map(|o| o.status.success())
                .unwrap_or(false);
            Self { use_systemd, handle: None }
        }
    }

    impl SystemListener for LinuxListener {
        fn start(&mut self, tx: mpsc::Sender<SystemEvent>) -> Result<()> {
            if self.use_systemd {
                self.start_systemd_listener(tx)
            } else {
                self.start_polling_listener(tx)
            }
        }

        fn stop(&mut self) -> Result<()> {
            if let Some(handle) = self.handle.take() {
                let _ = handle.join();
            }
            Ok(())
        }

        fn name(&self) -> &'static str { "linux" }
    }

    impl LinuxListener {
        fn start_systemd_listener(&mut self, tx: mpsc::Sender<SystemEvent>) -> Result<()> {
            // Use systemd's logind D-Bus interface for sleep/resume
            // For simplicity, we'll use a polling approach with loginctl
            // A full D-Bus implementation would be more robust
            let handle = thread::spawn(move || {
                let rt = tokio::runtime::Runtime::new().unwrap();
                rt.block_on(async {
                    let mut interval = interval(Duration::from_secs(2));
                    let mut last_idle_hint = false;
                    
                    loop {
                        interval.tick().await;
                        
                        // Check idle hint from logind
                        if let Ok(output) = Command::new("loginctl")
                            .args(["show-session", "$XDG_SESSION_ID", "--property=IdleHint"])
                            .output()
                        {
                            let stdout = String::from_utf8_lossy(&output.stdout);
                            let idle = stdout.contains("IdleHint=yes");
                            
                            // Detect transition from idle to active (wake)
                            if last_idle_hint && !idle {
                                info!("Linux: Wake detected via logind");
                                let _ = tx.send(SystemEvent::Wake);
                            }
                            last_idle_hint = idle;
                        }
                    }
                });
            });
            self.handle = Some(handle);
            Ok(())
        }

        fn start_polling_listener(&mut self, _tx: mpsc::Sender<SystemEvent>) -> Result<()> {
            // Fallback: monitor /proc/acpi/button/lid or similar
            // For now, just log that we're running
            info!("Linux: Using polling fallback for wake detection");
            let handle = thread::spawn(move || {
                // Could monitor /sys/power/state or use inotify on /var/log/pm-suspend.log
                loop {
                    thread::sleep(Duration::from_secs(30));
                }
            });
            self.handle = Some(handle);
            Ok(())
        }
    }
}

#[cfg(target_os = "linux")]
pub use linux::LinuxListener;

// ============================================================================
// macOS Implementation
// ============================================================================

#[cfg(target_os = "macos")]
pub mod macos {
    use super::*;
    use objc::{class, msg_send, sel, sel_impl};
    use objc::runtime::{Object, NO};
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;

    pub struct MacOSListener {
        running: Arc<AtomicBool>,
        observer: *mut Object,
        handle: Option<thread::JoinHandle<()>>,
    }

    impl MacOSListener {
        pub fn new() -> Self {
            Self { running: Arc::new(AtomicBool::new(false)), observer: std::ptr::null_mut(), handle: None }
        }
    }

    impl SystemListener for MacOSListener {
        fn start(&mut self, tx: mpsc::Sender<SystemEvent>) -> Result<()> {
            if self.running.swap(true, Ordering::SeqCst) {
                return Ok(());
            }

            let running = self.running.clone();
            let handle = thread::spawn(move || {
                // Register for NSWorkspaceDidWakeNotification
                unsafe {
                    let center: *mut Object = msg_send![class!(NSNotificationCenter), defaultCenter];
                    let queue: *mut Object = msg_send![class!(NSOperationQueue), mainQueue];
                    
                    let observer: *mut Object = msg_send![center, 
                        addObserverForName: sel!(NSWorkspaceDidWakeNotification)
                        object: nil
                        queue: queue
                        usingBlock: Box::new(move |_notification: *mut Object| {
                            if running.load(Ordering::SeqCst) {
                                info!("macOS: Wake notification received");
                                let _ = tx.send(SystemEvent::Wake);
                            }
                        })
                    ];
                    
                    // Also listen for sleep notification to track state
                    let _sleep_observer: *mut Object = msg_send![center,
                        addObserverForName: sel!(NSWorkspaceWillSleepNotification)
                        object: nil
                        queue: queue
                        usingBlock: Box::new(move |_| {
                            debug!("macOS: Sleep notification received");
                        })
                    ];
                    
                    // Keep thread alive
                    while running.load(Ordering::SeqCst) {
                        thread::sleep(std::time::Duration::from_secs(1));
                    }
                    
                    // Cleanup
                    let _: () = msg_send![center, removeObserver: observer];
                }
            });

            self.handle = Some(handle);
            Ok(())
        }

        fn stop(&mut self) -> Result<()> {
            self.running.store(false, Ordering::SeqCst);
            if let Some(handle) = self.handle.take() {
                let _ = handle.join();
            }
            Ok(())
        }

        fn name(&self) -> &'static str { "macos" }
    }
}

#[cfg(target_os = "macos")]
pub use macos::MacOSListener;

// ============================================================================
// Autostart/Boot Registration
// ============================================================================

/// Register/unregister application for autostart on login
pub mod autostart {
    use std::path::PathBuf;
    use anyhow::Result;
    use directories::ProjectDirs;

    #[derive(Debug, Clone, Copy)]
    pub enum Action {
        Enable,
        Disable,
    }

    /// Get the executable path
    fn exe_path() -> Result<PathBuf> {
        std::env::current_exe().map_err(Into::into)
    }

    /// Register for autostart on current platform
    pub fn set_autostart(action: Action) -> Result<()> {
        match action {
            Action::Enable => enable_autostart(),
            Action::Disable => disable_autostart(),
        }
    }

    #[cfg(target_os = "windows")]
    fn enable_autostart() -> Result<()> {
        use windows::Win32::System::Registry::*;
        use windows::core::PCWSTR;
        use std::ffi::OsStr;
        use std::os::windows::ffi::OsStrExt;

        let exe = exe_path()?;
        let path_str: Vec<u16> = exe.to_string_lossy().encode_utf16().chain(Some(0)).collect();
        
        unsafe {
            let subkey = OsStr::new("Software\\Microsoft\\Windows\\CurrentVersion\\Run").encode_utf16().chain(Some(0)).collect::<Vec<_>>();
            let value_name = OsStr::new("PostItRs").encode_utf16().chain(Some(0)).collect::<Vec<_>>();
            
            let mut key = HKEY::default();
            let result = RegOpenKeyExW(HKEY_CURRENT_USER, PCWSTR(subkey.as_ptr()), 0, KEY_SET_VALUE, &mut key);
            if result == ERROR_SUCCESS {
                let result = RegSetValueExW(
                    key,
                    PCWSTR(value_name.as_ptr()),
                    0,
                    REG_SZ,
                    path_str.as_ptr() as *const u8,
                    (path_str.len() * 2) as u32,
                );
                RegCloseKey(key);
                if result != ERROR_SUCCESS {
                    return Err(anyhow::anyhow!("Failed to set registry value: {}", result.0));
                }
            } else {
                return Err(anyhow::anyhow!("Failed to open registry key: {}", result.0));
            }
        }
        Ok(())
    }

    #[cfg(target_os = "windows")]
    fn disable_autostart() -> Result<()> {
        use windows::Win32::System::Registry::*;
        use windows::core::PCWSTR;
        use std::ffi::OsStr;
        use std::os::windows::ffi::OsStrExt;

        unsafe {
            let subkey = OsStr::new("Software\\Microsoft\\Windows\\CurrentVersion\\Run").encode_utf16().chain(Some(0)).collect::<Vec<_>>();
            let value_name = OsStr::new("PostItRs").encode_utf16().chain(Some(0)).collect::<Vec<_>>();
            
            let mut key = HKEY::default();
            let result = RegOpenKeyExW(HKEY_CURRENT_USER, PCWSTR(subkey.as_ptr()), 0, KEY_SET_VALUE, &mut key);
            if result == ERROR_SUCCESS {
                let result = RegDeleteValueW(key, PCWSTR(value_name.as_ptr()));
                RegCloseKey(key);
                if result != ERROR_SUCCESS && result != ERROR_FILE_NOT_FOUND {
                    return Err(anyhow::anyhow!("Failed to delete registry value: {}", result.0));
                }
            }
        }
        Ok(())
    }

    #[cfg(target_os = "linux")]
    fn enable_autostart() -> Result<()> {
        use std::fs;
        let proj = ProjectDirs::from("com", "postit-rs", "postit-rs").ok_or_else(|| anyhow::anyhow!("No config dir"))?;
        let autostart_dir = proj.config_dir().join("autostart");
        fs::create_dir_all(&autostart_dir)?;
        
        let exe = exe_path()?;
        let desktop_content = format!(
            r#"[Desktop Entry]
Type=Application
Name=PostItRs
Exec={}
Hidden=false
NoDisplay=false
X-GNOME-Autostart-enabled=true
"#,
            exe.display()
        );
        
        let desktop_path = autostart_dir.join("postit-rs.desktop");
        fs::write(desktop_path, desktop_content)?;
        Ok(())
    }

    #[cfg(target_os = "linux")]
    fn disable_autostart() -> Result<()> {
        let proj = ProjectDirs::from("com", "postit-rs", "postit-rs").ok_or_else(|| anyhow::anyhow!("No config dir"))?;
        let desktop_path = proj.config_dir().join("autostart").join("postit-rs.desktop");
        if desktop_path.exists() {
            std::fs::remove_file(desktop_path)?;
        }
        Ok(())
    }

    #[cfg(target_os = "macos")]
    fn enable_autostart() -> Result<()> {
        use std::fs;
        let proj = ProjectDirs::from("com", "postit-rs", "postit-rs").ok_or_else(|| anyhow::anyhow!("No config dir"))?;
        let launch_agents = dirs::home_dir().unwrap().join("Library/LaunchAgents");
        fs::create_dir_all(&launch_agents)?;
        
        let exe = exe_path()?;
        let plist_content = format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>com.postit-rs.postit</string>
    <key>ProgramArguments</key>
    <array>
        <string>{}</string>
    </array>
    <key>RunAtLoad</key>
    <true/>
    <key>KeepAlive</key>
    <false/>
</dict>
</plist>
"#,
            exe.display()
        );
        
        let plist_path = launch_agents.join("com.postit-rs.postit.plist");
        fs::write(plist_path, plist_content)?;
        Ok(())
    }

    #[cfg(target_os = "macos")]
    fn disable_autostart() -> Result<()> {
        let plist_path = dirs::home_dir().unwrap().join("Library/LaunchAgents/com.postit-rs.postit.plist");
        if plist_path.exists() {
            std::fs::remove_file(plist_path)?;
        }
        Ok(())
    }

    #[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
    fn enable_autostart() -> Result<()> {
        Err(anyhow::anyhow!("Autostart not supported on this platform"))
    }

    #[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
    fn disable_autostart() -> Result<()> {
        Err(anyhow::anyhow!("Autostart not supported on this platform"))
    }
}

pub use autostart::{set_autostart, Action as AutostartAction};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_system_event_enum() {
        assert_eq!(SystemEvent::Boot, SystemEvent::Boot);
        assert_ne!(SystemEvent::Boot, SystemEvent::Wake);
    }
}