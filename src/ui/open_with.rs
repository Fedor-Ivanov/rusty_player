//! Files opened with the app from the system: "Open With" / double click in Finder or Explorer,
//! a file dropped onto the Dock icon, `lmp song.mp3` from a terminal. Such files are played
//! right away (see `Destination::Play`).
//!
//! - Windows, Linux, a terminal: the paths come as command line arguments.
//! - Windows: Explorer starts a new process for every opened file, even when LMP is already
//!   running. Such a process hands its files over to the running one (`WM_COPYDATA`) and exits —
//!   otherwise a second player would start. The installer lists LMP in "Open with" for the
//!   audio types (`installer/windows.nsi`).
//! - macOS: Finder doesn't pass arguments — it sends the "open documents" Apple event, both at
//!   launch and to an app that is already running. winit doesn't handle it, so we install our own
//!   handler. The Info.plist must also list the audio types (`CFBundleDocumentTypes`,
//!   see `assets/macos/document_types.plist`) — otherwise Finder doesn't offer the app at all.
//!
//! The paths are put into a shared list; the app takes them every frame (`take`).

use eframe::egui;
use std::{
    path::PathBuf,
    sync::{Mutex, OnceLock},
};

/// Opened, but not yet taken by the app
static PENDING: Mutex<Vec<PathBuf>> = Mutex::new(Vec::new());
/// To wake the app when files arrive while it's idle
static CONTEXT: OnceLock<egui::Context> = OnceLock::new();

/// Call in `main` before the event loop starts: on macOS the "open documents" event of the
/// launch arrives before the first frame
pub fn install() {
    let args = std::env::args_os()
        .skip(1)
        .map(PathBuf::from)
        // Old macOS versions pass the process serial number (`-psn_0_12345`) to apps started by Finder
        .filter(|path| !path.to_string_lossy().starts_with("-psn_"))
        .filter(|path| path.exists());
    let paths: Vec<PathBuf> = args.collect();

    #[cfg(target_os = "windows")]
    if windows::hand_over_to_running(&paths) {
        std::process::exit(0);
    }

    push(paths);

    #[cfg(target_os = "macos")]
    macos::install();
}

/// The app window is created: from now on, files opened later wake the app right away
/// (on Windows — and they are accepted from other LMP processes)
pub(crate) fn attach(cc: &eframe::CreationContext) {
    let _ = CONTEXT.set(cc.egui_ctx.clone());

    #[cfg(target_os = "windows")]
    windows::receive_files(cc);
}

/// Files opened since the previous call
pub(crate) fn take() -> Vec<PathBuf> {
    std::mem::take(&mut *PENDING.lock().unwrap_or_else(|e| e.into_inner()))
}

fn push(paths: Vec<PathBuf>) {
    if paths.is_empty() {
        return;
    }
    PENDING
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .extend(paths);
    if let Some(ctx) = CONTEXT.get() {
        ctx.request_repaint();
    }
}

#[cfg(target_os = "macos")]
mod macos {
    use objc2::{MainThreadOnly, define_class, msg_send, rc::Retained, runtime::AnyObject, sel};
    use objc2_app_kit::NSApplicationWillFinishLaunchingNotification;
    use objc2_foundation::{
        MainThreadMarker, NSAppleEventDescriptor, NSAppleEventManager, NSNotification,
        NSNotificationCenter, NSObject,
    };
    use std::path::PathBuf;

    // Apple event codes (FourCharCode): the "open documents" event and its parameters
    /// 'aevt'
    const CORE_EVENT_CLASS: u32 = 0x6165_7674;
    /// 'odoc'
    const OPEN_DOCUMENTS: u32 = 0x6f64_6f63;
    /// '----' — the list of files
    const DIRECT_OBJECT: u32 = 0x2d2d_2d2d;
    /// 'furl'
    const FILE_URL: u32 = 0x6675_726c;

    define_class!(
        #[unsafe(super(NSObject))]
        #[thread_kind = MainThreadOnly]
        #[name = "LMPOpenFilesHandler"]
        struct Handler;

        impl Handler {
            /// AppKit installs its own Apple event handlers before this notification and
            /// delivers the launch's "open documents" event after it — so ours goes in here
            #[unsafe(method(applicationWillFinishLaunching:))]
            fn will_finish_launching(&self, _notification: &NSNotification) {
                let manager = NSAppleEventManager::sharedAppleEventManager();
                // SAFETY: the handler lives until the app exits (see `install`), the selector
                // is the method below with a matching signature
                unsafe {
                    manager.setEventHandler_andSelector_forEventClass_andEventID(
                        self,
                        sel!(openDocuments:withReplyEvent:),
                        CORE_EVENT_CLASS,
                        OPEN_DOCUMENTS,
                    );
                }
            }

            #[unsafe(method(openDocuments:withReplyEvent:))]
            fn open_documents(
                &self,
                event: &NSAppleEventDescriptor,
                _reply: &NSAppleEventDescriptor,
            ) {
                super::push(paths(event));
            }
        }
    );

    /// Paths of the files from the event: a list of file URLs
    fn paths(event: &NSAppleEventDescriptor) -> Vec<PathBuf> {
        let Some(list) = event.paramDescriptorForKeyword(DIRECT_OBJECT) else {
            return Vec::new();
        };
        // A single file can come as the descriptor itself rather than a list of one;
        // list items are numbered from 1
        let items: Vec<Retained<NSAppleEventDescriptor>> = match list.numberOfItems() {
            ..=0 => vec![list],
            count => (1..=count)
                .filter_map(|index| list.descriptorAtIndex(index))
                .collect(),
        };
        items
            .iter()
            .filter_map(|item| item.coerceToDescriptorType(FILE_URL)?.fileURLValue())
            .filter_map(|url| url.to_file_path())
            .collect()
    }

    pub(super) fn install() {
        let Some(mtm) = MainThreadMarker::new() else {
            return;
        };
        let handler: Retained<Handler> = unsafe { msg_send![Handler::alloc(mtm), init] };
        let center = NSNotificationCenter::defaultCenter();
        // SAFETY: the method exists and takes one notification
        unsafe {
            center.addObserver_selector_name_object(
                &handler,
                sel!(applicationWillFinishLaunching:),
                Some(NSApplicationWillFinishLaunchingNotification),
                None,
            );
        }
        // Neither the notification center nor the Apple event manager retain the handler —
        // it must live as long as the app
        let _: *mut AnyObject = Retained::into_raw(handler).cast();
    }
}

#[cfg(target_os = "windows")]
mod windows {
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};
    use std::{
        ffi::OsString,
        os::windows::ffi::{OsStrExt, OsStringExt},
        path::PathBuf,
        ptr,
        time::{Duration, Instant},
    };
    use windows_sys::{
        Win32::{
            Foundation::{ERROR_ALREADY_EXISTS, GetLastError, HWND, LPARAM, LRESULT, WPARAM},
            System::{DataExchange::COPYDATASTRUCT, Threading::CreateMutexW},
            UI::{
                Shell::{DefSubclassProc, SetWindowSubclass},
                WindowsAndMessaging::{
                    ChangeWindowMessageFilterEx, EnumWindows, GetPropW, IsIconic, MSGFLT_ALLOW,
                    SMTO_ABORTIFHUNG, SW_RESTORE, SendMessageTimeoutW, SetForegroundWindow,
                    SetPropW, ShowWindow, WM_COPYDATA,
                },
            },
        },
        core::BOOL,
    };

    /// Exists while some LMP process is running — a new process finds out it isn't the first
    const INSTANCE_MUTEX: &str = "Local\\LMP.instance";
    /// A mark on the LMP window: other processes find it by this property
    const WINDOW_PROP: &str = "LMP.OpenFiles";
    /// `WM_COPYDATA` kind: a list of paths, UTF-16, separated by zeros
    const OPEN_FILES: usize = 0x4c4d_5001;
    /// How long a new process waits for the window of the running one: when several files are
    /// opened at once, Explorer starts all the processes together, and the first one's window
    /// isn't there yet
    const WAIT_FOR_WINDOW: Duration = Duration::from_secs(5);
    const SEND_TIMEOUT_MS: u32 = 5000;

    fn wide(text: &str) -> Vec<u16> {
        text.encode_utf16().chain(Some(0)).collect()
    }

    /// If LMP is already running, gives it `paths`, brings its window to the front and returns `true` —
    /// then this process isn't needed. Without files a second window opens as before
    pub(super) fn hand_over_to_running(paths: &[PathBuf]) -> bool {
        let name = wide(INSTANCE_MUTEX);
        // SAFETY: the name is a zero-terminated string; the mutex is kept until the process exits
        let first = unsafe {
            let mutex = CreateMutexW(ptr::null(), 0, name.as_ptr());
            mutex.is_null() || GetLastError() != ERROR_ALREADY_EXISTS
        };
        if first || paths.is_empty() {
            return false;
        }

        let started = Instant::now();
        let window = loop {
            if let Some(window) = find_window() {
                break window;
            }
            if started.elapsed() >= WAIT_FOR_WINDOW {
                // The running one didn't show a window (hung or is exiting) — open here
                return false;
            }
            std::thread::sleep(Duration::from_millis(100));
        };

        let mut data: Vec<u16> = Vec::new();
        for path in paths {
            data.extend(path.as_os_str().encode_wide());
            data.push(0);
        }
        let message = COPYDATASTRUCT {
            dwData: OPEN_FILES,
            cbData: (data.len() * size_of::<u16>()) as u32,
            lpData: data.as_mut_ptr().cast(),
        };
        let mut result = 0;
        // SAFETY: `message` and `data` live until the call returns — WM_COPYDATA is synchronous
        let sent = unsafe {
            SendMessageTimeoutW(
                window,
                WM_COPYDATA,
                0,
                &message as *const COPYDATASTRUCT as LPARAM,
                SMTO_ABORTIFHUNG,
                SEND_TIMEOUT_MS,
                &mut result,
            )
        };
        if sent == 0 || result == 0 {
            return false;
        }
        // The process started by Explorer may bring a window to the front; the running one may not
        // SAFETY: a valid window handle
        unsafe {
            if IsIconic(window) != 0 {
                ShowWindow(window, SW_RESTORE);
            }
            SetForegroundWindow(window);
        }
        true
    }

    /// The window of the running LMP — the one with our property
    fn find_window() -> Option<HWND> {
        unsafe extern "system" fn check(window: HWND, found: LPARAM) -> BOOL {
            let prop = wide(WINDOW_PROP);
            // SAFETY: `found` is the `Option<HWND>` from `find_window`, alive during EnumWindows
            unsafe {
                if !GetPropW(window, prop.as_ptr()).is_null() {
                    *(found as *mut Option<HWND>) = Some(window);
                    return 0;
                }
            }
            1
        }
        let mut found: Option<HWND> = None;
        // SAFETY: the callback only writes into `found`
        unsafe { EnumWindows(Some(check), &mut found as *mut Option<HWND> as LPARAM) };
        found
    }

    /// Marks the app window and starts accepting files from other LMP processes
    pub(super) fn receive_files(cc: &eframe::CreationContext) {
        let Ok(handle) = cc.window_handle() else {
            return;
        };
        let RawWindowHandle::Win32(handle) = handle.as_raw() else {
            return;
        };
        let window = handle.hwnd.get() as HWND;
        let prop = wide(WINDOW_PROP);
        // SAFETY: the window lives as long as the app; the subclass procedure is a plain function
        unsafe {
            // LMP started by the installer runs as administrator: without this, messages from
            // processes started by Explorer wouldn't reach it
            ChangeWindowMessageFilterEx(window, WM_COPYDATA, MSGFLT_ALLOW, ptr::null_mut());
            SetWindowSubclass(window, Some(window_proc), 1, 0);
            SetPropW(window, prop.as_ptr(), 1 as _);
        }
    }

    unsafe extern "system" fn window_proc(
        window: HWND,
        message: u32,
        wparam: WPARAM,
        lparam: LPARAM,
        _id: usize,
        _data: usize,
    ) -> LRESULT {
        if message == WM_COPYDATA {
            // SAFETY: for WM_COPYDATA, lparam points to COPYDATASTRUCT, valid during the call
            let copy = unsafe { &*(lparam as *const COPYDATASTRUCT) };
            if copy.dwData == OPEN_FILES {
                let len = copy.cbData as usize / size_of::<u16>();
                let data: &[u16] = if len == 0 || copy.lpData.is_null() {
                    &[]
                } else {
                    // SAFETY: the sender passed `cbData` bytes of UTF-16
                    unsafe { std::slice::from_raw_parts(copy.lpData.cast::<u16>(), len) }
                };
                let paths = data
                    .split(|&unit| unit == 0)
                    .filter(|path| !path.is_empty())
                    .map(|path| PathBuf::from(OsString::from_wide(path)))
                    .collect();
                super::push(paths);
                return 1;
            }
        }
        // SAFETY: the rest goes to winit's window procedure
        unsafe { DefSubclassProc(window, message, wparam, lparam) }
    }
}
