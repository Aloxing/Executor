// Application foundation: window lifecycle, tray, settings persistence,
// WebView2 memory control and platform business kernels.
// Generic, app-agnostic helpers live in `crate::common` instead.
#![allow(dead_code)]

pub mod android;
pub mod settings;
pub mod tray;
pub mod webview;
pub mod window;
