use std::cell::{Cell, RefCell};
use std::ffi::c_void;
use std::ptr;
use std::rc::Rc;

use cocoa::appkit::{
    NSApp, NSApplication, NSApplicationActivationPolicyRegular, NSBackingStoreBuffered,
    NSPasteboard, NSView, NSWindow, NSWindowStyleMask,
};
use cocoa::base::{id, nil, NO, YES};
use cocoa::foundation::{NSAutoreleasePool, NSPoint, NSRect, NSSize, NSString};
use core_foundation::runloop::{
    __CFRunLoopTimer, kCFRunLoopCommonModes, CFRunLoop, CFRunLoopTimer, CFRunLoopTimerContext,
};
use keyboard_types::KeyboardEvent;

use objc::{msg_send, runtime::Object, sel, sel_impl};

use raw_window_handle::{
    AppKitDisplayHandle, AppKitWindowHandle, HasRawDisplayHandle, HasRawWindowHandle,
    RawDisplayHandle, RawWindowHandle,
};

use crate::{
    Event, EventStatus, MouseCursor, Size, WindowHandler, WindowInfo, WindowOpenOptions,
    WindowScalePolicy,
};

use super::keyboard::KeyboardState;
use super::view::{create_view, BASEVIEW_STATE_IVAR};

#[cfg(feature = "opengl")]
use crate::gl::{GlConfig, GlContext};

pub struct WindowHandle {
    state: Rc<WindowState>,
}

impl WindowHandle {
    pub fn close(&mut self) {
        self.state.window_inner.close();
    }

    pub fn is_open(&self) -> bool {
        self.state.window_inner.open.get()
    }
}

unsafe impl HasRawWindowHandle for WindowHandle {
    fn raw_window_handle(&self) -> RawWindowHandle {
        self.state.window_inner.raw_window_handle()
    }
}

pub(super) struct WindowInner {
    pub(super) open: Cell<bool>,

    /// Only set if we created the parent window, i.e. we are running in
    /// parentless mode
    ns_app: Cell<Option<id>>,
    /// Only set if we created the parent window, i.e. we are running in
    /// parentless mode
    ns_window: Cell<Option<id>>,
    /// Our subclassed NSView
    pub(super) ns_view: id,

    #[cfg(feature = "opengl")]
    pub(super) gl_context: Option<GlContext>,
}

impl WindowInner {
    pub(super) fn close(&self) {
        if self.open.get() {
            self.open.set(false);

            unsafe {
                // Take back ownership of the NSView's Rc<WindowState>
                let state_ptr: *const c_void = *(*self.ns_view).get_ivar(BASEVIEW_STATE_IVAR);
                // Clear the ivar first so setFrameSize / backing-change
                // callbacks during teardown cannot resurrect a freed Rc.
                (*self.ns_view).set_ivar(BASEVIEW_STATE_IVAR, ptr::null::<c_void>());
                let window_state = Rc::from_raw(state_ptr as *mut WindowState);

                // Cancel the frame timer
                if let Some(frame_timer) = window_state.frame_timer.take() {
                    CFRunLoop::get_current().remove_timer(&frame_timer, kCFRunLoopCommonModes);
                }

                drop(window_state);

                // Close the window if in non-parented mode
                if let Some(ns_window) = self.ns_window.take() {
                    ns_window.close();
                }

                // Ensure that the NSView is detached from the parent window
                self.ns_view.removeFromSuperview();
                let () = msg_send![self.ns_view as id, release];

                // If in non-parented mode, we want to also quit the app altogether
                let app = self.ns_app.take();
                if let Some(app) = app {
                    app.stop_(app);
                }
            }
        }
    }

    fn raw_window_handle(&self) -> RawWindowHandle {
        if self.open.get() {
            let ns_window = self.ns_window.get().unwrap_or(ptr::null_mut()) as *mut c_void;

            let mut handle = AppKitWindowHandle::empty();
            handle.ns_window = ns_window;
            handle.ns_view = self.ns_view as *mut c_void;

            return RawWindowHandle::AppKit(handle);
        }

        RawWindowHandle::AppKit(AppKitWindowHandle::empty())
    }
}

pub struct Window<'a> {
    inner: &'a WindowInner,
}

impl<'a> Window<'a> {
    pub fn open_parented<P, H, B>(parent: &P, options: WindowOpenOptions, build: B) -> WindowHandle
    where
        P: HasRawWindowHandle,
        H: WindowHandler + 'static,
        B: FnOnce(&mut crate::Window) -> H,
        B: Send + 'static,
    {
        let pool = unsafe { NSAutoreleasePool::new(nil) };

        let scaling = match options.scale {
            WindowScalePolicy::ScaleFactor(scale) => scale,
            WindowScalePolicy::SystemScaleFactor => 1.0,
        };

        let window_info = WindowInfo::from_logical_size(options.size, scaling);

        let handle = if let RawWindowHandle::AppKit(handle) = parent.raw_window_handle() {
            handle
        } else {
            panic!("Not a macOS window");
        };

        let ns_view = unsafe { create_view(&options) };

        let window_inner = WindowInner {
            open: Cell::new(true),
            ns_app: Cell::new(None),
            ns_window: Cell::new(None),
            ns_view,

            #[cfg(feature = "opengl")]
            gl_context: options
                .gl_config
                .map(|gl_config| Self::create_gl_context(None, ns_view, gl_config)),
        };

        let window_handle = Self::init(window_inner, window_info, build);

        unsafe {
            let _: id = msg_send![handle.ns_view as *mut Object, addSubview: ns_view];

            let () = msg_send![pool, drain];
        }

        window_handle
    }

    pub fn open_blocking<H, B>(options: WindowOpenOptions, build: B)
    where
        H: WindowHandler + 'static,
        B: FnOnce(&mut crate::Window) -> H,
        B: Send + 'static,
    {
        let pool = unsafe { NSAutoreleasePool::new(nil) };

        // It seems prudent to run NSApp() here before doing other
        // work. It runs [NSApplication sharedApplication], which is
        // what is run at the very start of the Xcode-generated main
        // function of a cocoa app according to:
        // https://developer.apple.com/documentation/appkit/nsapplication
        let app = unsafe { NSApp() };

        unsafe {
            app.setActivationPolicy_(NSApplicationActivationPolicyRegular);
        }

        let scaling = match options.scale {
            WindowScalePolicy::ScaleFactor(scale) => scale,
            WindowScalePolicy::SystemScaleFactor => 1.0,
        };

        let window_info = WindowInfo::from_logical_size(options.size, scaling);

        let rect = NSRect::new(
            NSPoint::new(0.0, 0.0),
            NSSize::new(window_info.logical_size().width, window_info.logical_size().height),
        );

        let ns_window = unsafe {
            let ns_window = NSWindow::alloc(nil).initWithContentRect_styleMask_backing_defer_(
                rect,
                NSWindowStyleMask::NSTitledWindowMask
                    | NSWindowStyleMask::NSClosableWindowMask
                    | NSWindowStyleMask::NSMiniaturizableWindowMask,
                NSBackingStoreBuffered,
                NO,
            );
            ns_window.center();

            let title = NSString::alloc(nil).init_str(&options.title).autorelease();
            ns_window.setTitle_(title);

            ns_window.makeKeyAndOrderFront_(nil);

            ns_window
        };

        let ns_view = unsafe { create_view(&options) };

        let window_inner = WindowInner {
            open: Cell::new(true),
            ns_app: Cell::new(Some(app)),
            ns_window: Cell::new(Some(ns_window)),
            ns_view,

            #[cfg(feature = "opengl")]
            gl_context: options
                .gl_config
                .map(|gl_config| Self::create_gl_context(Some(ns_window), ns_view, gl_config)),
        };

        let _ = Self::init(window_inner, window_info, build);

        unsafe {
            ns_window.setContentView_(ns_view);
            ns_window.setDelegate_(ns_view);

            let () = msg_send![pool, drain];

            app.run();
        }
    }

    fn init<H, B>(window_inner: WindowInner, window_info: WindowInfo, build: B) -> WindowHandle
    where
        H: WindowHandler + 'static,
        B: FnOnce(&mut crate::Window) -> H,
        B: Send + 'static,
    {
        let mut window = crate::Window::new(Window { inner: &window_inner });
        let window_handler = Box::new(build(&mut window));

        let ns_view = window_inner.ns_view;

        let window_state = Rc::new(WindowState {
            window_inner,
            window_handler: RefCell::new(window_handler),
            keyboard_state: KeyboardState::new(),
            frame_timer: Cell::new(None),
            window_info: Cell::new(window_info),
            pending_resize: Cell::new(None),
            host_bounds: Cell::new(None),
        });

        let window_state_ptr = Rc::into_raw(Rc::clone(&window_state));

        unsafe {
            (*ns_view).set_ivar(BASEVIEW_STATE_IVAR, window_state_ptr as *const c_void);

            WindowState::setup_timer(window_state_ptr);
        }

        WindowHandle { state: window_state }
    }

    pub fn close(&mut self) {
        self.inner.close();
    }

    pub fn resize(&mut self, size: Size) {
        if self.inner.open.get() {
            // NOTE: macOS gives you a personal rave if you pass in fractional pixels here. Even
            // though the size is in fractional pixels.
            let width = size.width.round().max(0.0);
            let height = size.height.round().max(0.0);
            if width < 1.0 || height < 1.0 {
                return;
            }
            let size = NSSize::new(width, height);

            // `setFrameSize:` is overridden to sync OpenGL and emit Resized.
            unsafe { NSView::setFrameSize(self.inner.ns_view, size) };
            // Embedded hosts commonly use unflipped coordinates. Origin zero is
            // their bottom edge, so a fitted editor must move up to remain at top.
            if self.inner.ns_window.get().is_none() {
                unsafe {
                    let parent: id = msg_send![self.inner.ns_view, superview];
                    if parent != nil {
                        let bounds = NSView::bounds(parent);
                        let flipped: cocoa::base::BOOL = msg_send![parent, isFlipped];
                        let y = if flipped == YES {
                            bounds.origin.y
                        } else {
                            bounds.origin.y + bounds.size.height - height
                        };
                        let _: () = msg_send![self.inner.ns_view,
                            setFrameOrigin: NSPoint::new(bounds.origin.x, y)];
                    }
                }
            }

            unsafe {
                let _: () = msg_send![self.inner.ns_view, setNeedsDisplay: YES];
            }

            // If this is a standalone window then we'll also need to resize the window itself
            if let Some(ns_window) = self.inner.ns_window.get() {
                unsafe { NSWindow::setContentSize_(ns_window, size) };
            }
        }
    }

    pub fn set_mouse_cursor(&mut self, _mouse_cursor: MouseCursor) {
        // Cursor changes are not implemented for this backend. A `todo!()`
        // here aborted the DAW with no plugin log.
    }

    #[cfg(feature = "opengl")]
    pub fn gl_context(&self) -> Option<&GlContext> {
        self.inner.gl_context.as_ref()
    }

    #[cfg(feature = "opengl")]
    fn create_gl_context(ns_window: Option<id>, ns_view: id, config: GlConfig) -> GlContext {
        let mut handle = AppKitWindowHandle::empty();
        handle.ns_window = ns_window.unwrap_or(ptr::null_mut()) as *mut c_void;
        handle.ns_view = ns_view as *mut c_void;
        let handle = RawWindowHandle::AppKit(handle);

        unsafe { GlContext::create(&handle, config).expect("Could not create OpenGL context") }
    }
}

pub(super) struct WindowState {
    pub(super) window_inner: WindowInner,
    window_handler: RefCell<Box<dyn WindowHandler>>,
    keyboard_state: KeyboardState,
    frame_timer: Cell<Option<CFRunLoopTimer>>,
    /// The last known window info for this window.
    pub window_info: Cell<WindowInfo>,
    pending_resize: Cell<Option<WindowInfo>>,
    /// Host space is independent of the smaller, aspect-fitted child view.
    host_bounds: Cell<Option<(f64, f64)>>,
}

impl WindowState {
    /// Gets the `WindowState` held by a given `NSView`.
    ///
    /// This method returns a cloned `Rc<WindowState>` rather than just a `&WindowState`, since the
    /// original `Rc<WindowState>` owned by the `NSView` can be dropped at any time
    /// (including during an event handler).
    pub(super) unsafe fn from_view(view: &Object) -> Rc<WindowState> {
        Self::try_from_view(view).expect("Baseview window state has not been initialized")
    }

    /// Returns the `WindowState` held by a given `NSView`, if initialization has completed.
    pub(super) unsafe fn try_from_view(view: &Object) -> Option<Rc<WindowState>> {
        let state_ptr: *const c_void = *view.get_ivar(BASEVIEW_STATE_IVAR);
        if state_ptr.is_null() {
            return None;
        }

        let state_rc = Rc::from_raw(state_ptr as *const WindowState);
        let state = Rc::clone(&state_rc);
        let _ = Rc::into_raw(state_rc);

        Some(state)
    }

    pub(super) fn trigger_event(&self, event: Event) -> EventStatus {
        let mut window = crate::Window::new(Window { inner: &self.window_inner });
        self.window_handler.borrow_mut().on_event(&mut window, event)
    }

    fn flush_pending_resize(&self) {
        let Some(info) = self.pending_resize.take() else {
            return;
        };
        let Ok(mut handler) = self.window_handler.try_borrow_mut() else {
            self.pending_resize.set(Some(info));
            return;
        };
        // Change the native drawable and renderer dimensions together, before
        // drawing. Resizing GL inside an AppKit callback used to leave Vizia
        // drawing an old-height viewport at the bottom of the new surface.
        #[cfg(feature = "opengl")]
        if let Some(gl_context) = &self.window_inner.gl_context {
            let size = info.logical_size();
            gl_context.resize(NSSize::new(size.width.round(), size.height.round()));
        }
        self.window_info.set(info);
        let mut window = crate::Window::new(Window { inner: &self.window_inner });
        handler.on_event(&mut window, Event::Window(crate::WindowEvent::Resized(info)));
    }

    /// Keep OpenGL and vizia in sync with the NSView frame. Skip 0×0
    /// sizes: FemtoVG `canvas.set_size(0, 0)` aborts native GL.
    pub(super) fn sync_from_view(&self) {
        if !self.window_inner.open.get() {
            return;
        }

        let (bounds, scale_factor) = unsafe {
            let ns_window: *mut Object = msg_send![self.window_inner.ns_view, window];
            let scale_factor: f64 =
                if ns_window.is_null() { 1.0 } else { NSWindow::backingScaleFactor(ns_window) };
            let bounds: NSRect = msg_send![self.window_inner.ns_view, bounds];
            (bounds, scale_factor)
        };

        if !bounds.size.width.is_finite()
            || !bounds.size.height.is_finite()
            || bounds.size.width < 1.0
            || bounds.size.height < 1.0
        {
            return;
        }

        let new_window_info = WindowInfo::from_logical_size(
            Size::new(bounds.size.width, bounds.size.height),
            scale_factor,
        );
        if new_window_info.physical_size().width == 0 || new_window_info.physical_size().height == 0
        {
            return;
        }
        let current = self.window_info.get();
        let changed = new_window_info.physical_size() != current.physical_size()
            || new_window_info.scale() != current.scale();
        // Coalesce all native callbacks to the newest valid size. Never touch
        // the drawable while the handler may be rendering or processing events.
        self.pending_resize.set(changed.then_some(new_window_info));
    }

    /// Autoresizing an aspect-fitted child only adds the parent's size delta.
    /// Its unused margins therefore become permanent, preventing growth on the
    /// other axis. Observe the actual host area instead, once per frame, and
    /// deliver its full dimensions whenever that area changes. Plugin-driven
    /// fitting never changes this cached host size.
    fn sync_host_bounds(&self) {
        if self.window_inner.ns_window.get().is_some() {
            return;
        }
        unsafe {
            let parent: id = msg_send![self.window_inner.ns_view, superview];
            if parent == nil {
                return;
            }
            let bounds = NSView::bounds(parent);
            let size = (bounds.size.width, bounds.size.height);
            if !size.0.is_finite() || !size.1.is_finite() || size.0 < 1.0 || size.1 < 1.0 {
                return;
            }
            if self.host_bounds.get() == Some(size) {
                return;
            }
            self.host_bounds.set(Some(size));
            // This only queues the resize. GL and Vizia still update together
            // through flush_pending_resize(), outside native view callbacks.
            NSView::setFrameSize(self.window_inner.ns_view, bounds.size);
            let _: () = msg_send![self.window_inner.ns_view, setFrameOrigin: bounds.origin];
            self.sync_from_view();
        }
    }

    pub(super) fn trigger_frame(&self) {
        if !self.window_inner.open.get() {
            return;
        }
        self.sync_host_bounds();
        unsafe {
            let bounds: NSRect = msg_send![self.window_inner.ns_view, bounds];
            if !bounds.size.width.is_finite()
                || !bounds.size.height.is_finite()
                || bounds.size.width < 1.0
                || bounds.size.height < 1.0
            {
                return;
            }
        }
        self.flush_pending_resize();
        let Ok(mut handler) = self.window_handler.try_borrow_mut() else {
            return;
        };
        let mut window = crate::Window::new(Window { inner: &self.window_inner });
        handler.on_frame(&mut window);
    }

    pub(super) fn keyboard_state(&self) -> &KeyboardState {
        &self.keyboard_state
    }

    pub(super) fn process_native_key_event(&self, event: *mut Object) -> Option<KeyboardEvent> {
        self.keyboard_state.process_native_event(event)
    }

    unsafe fn setup_timer(window_state_ptr: *const WindowState) {
        extern "C" fn timer_callback(_: *mut __CFRunLoopTimer, window_state_ptr: *mut c_void) {
            unsafe {
                let window_state = &*(window_state_ptr as *const WindowState);

                window_state.trigger_frame();
            }
        }

        let mut timer_context = CFRunLoopTimerContext {
            version: 0,
            info: window_state_ptr as *mut c_void,
            retain: None,
            release: None,
            copyDescription: None,
        };

        let timer = CFRunLoopTimer::new(0.0, 0.015, 0, 0, timer_callback, &mut timer_context);

        CFRunLoop::get_current().add_timer(&timer, kCFRunLoopCommonModes);

        (*window_state_ptr).frame_timer.set(Some(timer));
    }
}

unsafe impl<'a> HasRawWindowHandle for Window<'a> {
    fn raw_window_handle(&self) -> RawWindowHandle {
        self.inner.raw_window_handle()
    }
}

unsafe impl<'a> HasRawDisplayHandle for Window<'a> {
    fn raw_display_handle(&self) -> RawDisplayHandle {
        RawDisplayHandle::AppKit(AppKitDisplayHandle::empty())
    }
}

pub fn copy_to_clipboard(string: &str) {
    unsafe {
        let pb = NSPasteboard::generalPasteboard(nil);

        let ns_str = NSString::alloc(nil).init_str(string);

        pb.clearContents();
        pb.setString_forType(ns_str, cocoa::appkit::NSPasteboardTypeString);
    }
}

/// AppKit screen geometry is only accessed on the main thread. The visible frame
/// excludes the Dock and menu bar and uses points rather than Retina pixels.
pub fn available_screen_size() -> Option<Size> {
    unsafe {
        let is_main: cocoa::base::BOOL = msg_send![objc::class!(NSThread), isMainThread];
        if is_main != YES {
            return None;
        }
        let screen: id = msg_send![objc::class!(NSScreen), mainScreen];
        if screen == nil {
            return None;
        }
        let frame: NSRect = msg_send![screen, visibleFrame];
        Some(Size::new(frame.size.width, frame.size.height))
    }
}
