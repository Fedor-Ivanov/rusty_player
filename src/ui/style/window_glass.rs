//! Real glass under the window: the window is transparent, and the system blurs the desktop
//! and the windows behind it.

/// Enables the system blur under the window. If the effect is unavailable, the window just stays
/// transparent — that's no reason to crash, so the error is only returned.
pub fn apply(cc: &eframe::CreationContext<'_>) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    return macos::apply(cc);

    #[cfg(target_os = "windows")]
    return windows::apply(cc);

    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        let _ = cc;
        Err("window blur is only supported on macOS and Windows".into())
    }
}

/// What to render the window with. On Windows — OpenGL (glow): it renders into the regular window surface,
/// which winit makes transparent, and the system Acrylic shows under it. wgpu on Windows renders
/// via DX12: directly into the window — without transparency, or via DirectComposition — under
/// the window surface, which covers it (it can only be removed with a flag at window
/// creation, and eframe doesn't pass it). On macOS — wgpu, as by default
pub fn renderer() -> eframe::Renderer {
    #[cfg(target_os = "windows")]
    return eframe::Renderer::Glow;
    #[cfg(not(target_os = "windows"))]
    eframe::Renderer::default()
}

/// Windows: the glass is the system Acrylic material (blurs windows and the desktop behind),
/// in the dark variant. Not Mica: Mica isn't see-through — it's an almost solid fill matching
/// the wallpaper, and in the Windows light theme it's light too
#[cfg(target_os = "windows")]
mod windows {
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};
    use windows_sys::Win32::{
        Foundation::HWND,
        Graphics::Dwm::{
            DWMSBT_TRANSIENTWINDOW, DWMWA_SYSTEMBACKDROP_TYPE, DWMWA_USE_IMMERSIVE_DARK_MODE,
            DwmSetWindowAttribute,
        },
    };

    pub fn apply(cc: &eframe::CreationContext<'_>) -> Result<(), String> {
        let handle = cc.window_handle().map_err(|e| e.to_string())?;
        let RawWindowHandle::Win32(handle) = handle.as_raw() else {
            return Err("the window is not a Win32 window".into());
        };
        let hwnd = handle.hwnd.get() as HWND;
        // Dark variant of the material and title bar — regardless of the Windows theme
        let _ = set_attribute(hwnd, DWMWA_USE_IMMERSIVE_DARK_MODE, 1u32);
        // System Acrylic exists in Windows 11 22H2 and newer; the system draws it itself,
        // the window doesn't lag while dragging. Without it (Windows 10) — a regular blur:
        // Windows 10 Acrylic noticeably lags while dragging the window
        if set_attribute(hwnd, DWMWA_SYSTEMBACKDROP_TYPE, DWMSBT_TRANSIENTWINDOW).is_ok() {
            return Ok(());
        }
        window_vibrancy::apply_blur(cc, None).map_err(|e| e.to_string())
    }

    /// A window attribute for DWM. Error — the system doesn't know it (an old Windows version)
    fn set_attribute<T>(hwnd: HWND, attribute: i32, value: T) -> Result<(), i32> {
        // SAFETY: the window is alive, `value` lives until the end of the call, the size is its own
        let result = unsafe {
            DwmSetWindowAttribute(
                hwnd,
                attribute as u32,
                (&raw const value).cast(),
                size_of::<T>() as u32,
            )
        };
        if result < 0 { Err(result) } else { Ok(()) }
    }
}

#[cfg(target_os = "macos")]
mod macos {
    use objc2::MainThreadMarker;
    use objc2_app_kit::{
        NSAppearance, NSAppearanceCustomization, NSAppearanceNameDarkAqua,
        NSAutoresizingMaskOptions, NSView, NSVisualEffectBlendingMode, NSVisualEffectMaterial,
        NSVisualEffectState, NSVisualEffectView, NSWindowOrderingMode,
    };
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};

    /// NSVisualEffectView is placed not inside the egui view but next to it — in the window frame, under it.
    /// The winit view draws egui directly into its layer (Metal), and AppKit always draws subviews
    /// on top of the parent's layer: a blur inside the egui view would cover all the content
    /// (window-vibrancy does that — fine for Tauri, not for egui).
    /// The egui view itself stays in place, so input is unaffected.
    pub fn apply(cc: &eframe::CreationContext<'_>) -> Result<(), String> {
        let mtm = MainThreadMarker::new()
            .ok_or("window blur can only be enabled from the main thread")?;

        let handle = cc.window_handle().map_err(|e| e.to_string())?;
        let RawWindowHandle::AppKit(handle) = handle.as_raw() else {
            return Err("the window is not an AppKit window".into());
        };
        // SAFETY: the view belongs to the window, which is alive for the whole app lifetime
        let content_view: &NSView = unsafe { handle.ns_view.cast().as_ref() };
        // SAFETY: the window frame lives as long as the window
        let frame_view =
            unsafe { content_view.superview() }.ok_or("the window view has no frame view")?;

        let blur = NSVisualEffectView::initWithFrame(mtm.alloc(), frame_view.bounds());
        // HUDWindow — a dark material for the dark theme
        blur.setMaterial(NSVisualEffectMaterial::HUDWindow);
        // Dark appearance is forced, like macOS widgets: otherwise the material follows
        // the system theme and becomes light and milky in the light theme
        // SAFETY: a system constant, available for the whole app lifetime
        let dark = NSAppearance::appearanceNamed(unsafe { NSAppearanceNameDarkAqua });
        blur.setAppearance(dark.as_deref());
        blur.setBlendingMode(NSVisualEffectBlendingMode::BehindWindow);
        // Active — the blur doesn't fade when the window is out of focus
        blur.setState(NSVisualEffectState::Active);
        blur.setAutoresizingMask(
            NSAutoresizingMaskOptions::ViewWidthSizable
                | NSAutoresizingMaskOptions::ViewHeightSizable,
        );
        frame_view.addSubview_positioned_relativeTo(
            &blur,
            NSWindowOrderingMode::Below,
            Some(content_view),
        );
        Ok(())
    }
}
