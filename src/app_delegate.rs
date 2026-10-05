//! The Objective-C object that owns application state and receives all UI callbacks.
//!
//! Everything here is main-thread-only (`NSMenuDelegate` requires it), so interior mutability
//! uses `RefCell` rather than a lock — there is no second thread to contend with.

use std::cell::RefCell;

use objc2::rc::Retained;
use objc2::runtime::{AnyObject, ProtocolObject};
use objc2::{DefinedClass, MainThreadOnly, define_class, msg_send, sel};
use objc2_app_kit::{NSApplication, NSButton, NSMenu, NSMenuDelegate, NSStatusItem};
use objc2_foundation::{MainThreadMarker, NSObject, NSObjectProtocol, NSTimer};

use crate::killer::{self, KillOutcome};
use crate::ports::{self, PortEntry};
use crate::feedback::{alert, copy_to_clipboard};
use crate::{menu_builder, status_item};

/// How often the menu bar count is refreshed while the menu is closed.
const POLL_INTERVAL_SECS: f64 = 300.0;

pub struct DelegateIvars {
    /// Snapshot backing the currently rendered rows. Kill button tags index into this, so it must
    /// be replaced in the same operation that rebuilds the rows.
    entries: RefCell<Vec<PortEntry>>,
    /// Set when port discovery fails, so the menu can explain itself instead of looking empty.
    error: RefCell<Option<String>>,
    status_item: Retained<NSStatusItem>,
    menu: Retained<NSMenu>,
}

define_class!(
    // SAFETY:
    // - NSObject imposes no subclassing requirements.
    // - Delegate does not implement Drop.
    #[unsafe(super(NSObject))]
    // NSMenuDelegate is main-thread-only, so the class must be too.
    #[thread_kind = MainThreadOnly]
    #[name = "PortZapDelegate"]
    #[ivars = DelegateIvars]
    pub struct Delegate;

    impl Delegate {
        /// Kill the process for the clicked row. The button's tag is its index in the snapshot.
        #[unsafe(method(killClicked:))]
        fn kill_clicked(&self, sender: &NSButton) {
            let mtm = MainThreadMarker::from(self);

            // Clone out of the RefCell before killing: the handler re-enters ivars to reload,
            // and holding a borrow across that would panic.
            let entry = self
                .ivars()
                .entries
                .borrow()
                .get(sender.tag() as usize)
                .cloned();
            let Some(entry) = entry else { return };

            match killer::kill_process(entry.pid) {
                KillOutcome::Killed => {}
                KillOutcome::PermissionDenied => {
                    let command = killer::sudo_command_for(entry.pid);
                    let body = if copy_to_clipboard(&command) {
                        format!("`{command}` has been copied to your clipboard.")
                    } else {
                        format!("Run `{command}` in a terminal.")
                    };
                    alert(
                        &format!("Can't kill {} (PID {})", entry.process, entry.pid),
                        &format!("It belongs to another user.\n\n{body}"),
                        mtm,
                    );
                }
                KillOutcome::Failed(message) => {
                    alert(
                        &format!("Failed to kill {} (PID {})", entry.process, entry.pid),
                        &message,
                        mtm,
                    );
                }
            }

            self.reload(mtm);
        }

        #[unsafe(method(refreshClicked:))]
        fn refresh_clicked(&self, _sender: Option<&AnyObject>) {
            self.reload(MainThreadMarker::from(self));
        }

        #[unsafe(method(quitClicked:))]
        fn quit_clicked(&self, _sender: Option<&AnyObject>) {
            let mtm = MainThreadMarker::from(self);
            NSApplication::sharedApplication(mtm).terminate(None);
        }

        /// Periodic tick. Only the count matters here — rows are rebuilt when the menu opens,
        /// so building them now would construct a view per port that nobody is looking at.
        #[unsafe(method(timerFired:))]
        fn timer_fired(&self, _timer: Option<&AnyObject>) {
            self.refresh_entries(MainThreadMarker::from(self));
        }
    }

    unsafe impl NSObjectProtocol for Delegate {}

    // Rebuilding on open is what keeps rows fresh: the 5-minute tick alone would let a user see
    // minutes-old data, since it cannot know when the menu is about to appear.
    unsafe impl NSMenuDelegate for Delegate {
        #[unsafe(method(menuNeedsUpdate:))]
        fn menu_needs_update(&self, _menu: &NSMenu) {
            self.reload(MainThreadMarker::from(self));
        }
    }
);

impl Delegate {
    pub fn new(mtm: MainThreadMarker) -> Retained<Self> {
        let this = Self::alloc(mtm).set_ivars(DelegateIvars {
            entries: RefCell::new(Vec::new()),
            error: RefCell::new(None),
            status_item: status_item::create_status_item(mtm),
            menu: NSMenu::new(mtm),
        });
        let this: Retained<Self> = unsafe { msg_send![super(this), init] };

        let ivars = this.ivars();
        ivars.menu.setAutoenablesItems(false);
        ivars
            .menu
            .setDelegate(Some(ProtocolObject::from_ref(&*this)));
        ivars.status_item.setMenu(Some(&ivars.menu));

        // SAFETY: target outlives the timer (both live for the process lifetime) and
        // `timerFired:` is defined above.
        unsafe {
            NSTimer::scheduledTimerWithTimeInterval_target_selector_userInfo_repeats(
                POLL_INTERVAL_SECS,
                &*this,
                sel!(timerFired:),
                None,
                true,
            );
        }

        // Only the count is needed now; the rows are built when the menu is first opened.
        this.refresh_entries(mtm);
        this
    }

    /// Re-read the port list and update the menu bar count.
    ///
    /// Does not touch the menu, so it is cheap enough to run on a timer while the menu is closed.
    fn refresh_entries(&self, mtm: MainThreadMarker) {
        let ivars = self.ivars();

        match ports::list_listening_ports() {
            Ok(entries) => {
                status_item::set_count(&ivars.status_item, entries.len(), mtm);
                *ivars.error.borrow_mut() = None;
                *ivars.entries.borrow_mut() = entries;
            }
            Err(error) => {
                status_item::set_warning(&ivars.status_item, mtm);
                *ivars.error.borrow_mut() = Some(error.to_string());
                ivars.entries.borrow_mut().clear();
            }
        }
    }

    /// Re-read the port list and rebuild the menu rows from it.
    ///
    /// Snapshot and rows are always replaced together so a Kill button's tag can never index a
    /// stale entry.
    fn reload(&self, mtm: MainThreadMarker) {
        self.refresh_entries(mtm);

        let ivars = self.ivars();
        let target: &AnyObject = self.as_ref();
        menu_builder::build_menu(
            &ivars.menu,
            &ivars.entries.borrow(),
            ivars.error.borrow().as_deref(),
            target,
            mtm,
        );
    }
}
