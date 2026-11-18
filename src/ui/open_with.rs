//! Files opened with the app from the system: "Open With" / double click in Finder or Explorer,
//! a file dropped onto the Dock icon, `lmp song.mp3` from a terminal. Such files are played
//! right away (see `Destination::Play`).
//!
//! - Windows, Linux, a terminal: the paths come as command line arguments.
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
    push(args.collect());

    #[cfg(target_os = "macos")]
    macos::install();
}

/// The app is created: from now on, files opened later wake it right away
pub(crate) fn set_context(ctx: &egui::Context) {
    let _ = CONTEXT.set(ctx.clone());
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
