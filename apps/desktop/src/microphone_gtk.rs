//! The microphone, for the composer's voice messages, on WebKitGTK.
//!
//! WebKitGTK ships with media capture off and refuses every request for it,
//! and wry turns neither on. So devpit's own window — never a browser pane's
//! page, which is someone else's site — gets capture, and a request for the
//! microphone alone is let through. A camera, or the screen, stays refused.

use webkit2gtk::glib::object::Cast;
use webkit2gtk::{
    PermissionRequestExt, SettingsExt, UserMediaPermissionRequest, UserMediaPermissionRequestExt,
    WebViewExt,
};

pub fn allow(window: &tauri::WebviewWindow) {
    let _ = window.with_webview(|platform| {
        let view = platform.inner();
        if let Some(settings) = WebViewExt::settings(&view) {
            settings.set_enable_media_stream(true);
        }
        view.connect_permission_request(|_, request| {
            match request.downcast_ref::<UserMediaPermissionRequest>() {
                Some(media) if media.is_for_audio_device() && !media.is_for_video_device() => {
                    request.allow();
                    true
                }
                _ => false,
            }
        });
    });
}
