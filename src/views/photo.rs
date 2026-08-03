//! Choosing a photo, either through the camera or from the device.

use crate::camera::{self, CameraSession};
use base64::{engine::general_purpose::STANDARD, Engine as _};
use dioxus::prelude::*;

/// Largest photo the picker will accept from a file, mirroring the server's own
/// limit so an oversized one is rejected before it is read and encoded rather
/// than after a round trip. The server enforces the same ceiling regardless.
const MAX_PHOTO_BYTES: u64 = 15 * 1024 * 1024;

/// Produces a photo as `(mime, base64)`, from the camera or from a file.
///
/// The two routes sit alongside each other rather than one standing in for the
/// other. The file input is always present — it reaches photos taken earlier,
/// and on a phone it also offers the camera through the operating system. The
/// viewfinder button appears in addition to it, wherever `getUserMedia` can
/// actually be used.
#[component]
pub fn PhotoPicker(
    on_photo: EventHandler<(String, String)>,
    on_error: EventHandler<String>,
) -> Element {
    // Probed after the first render rather than during it. The server has no
    // navigator, so deciding this while rendering would give the client a first
    // render that disagreed with the server's — a hydration mismatch.
    let mut camera_available = use_signal(|| false);
    use_effect(move || camera_available.set(camera::is_supported()));

    let mut viewfinder_open = use_signal(|| false);
    let mut session = use_signal(CameraSession::new);
    let mut starting = use_signal(|| false);

    // Dismissing the viewfinder and releasing the camera are the same act, so
    // the indicator light never outlives the preview.
    let mut close_viewfinder = move || {
        session.write().close();
        viewfinder_open.set(false);
    };

    let start_camera = move |event: MountedEvent| async move {
        starting.set(true);

        // Opened into a local session and only then stored. Holding the signal
        // borrowed across the await would collide with any other read of it.
        let mut opening = CameraSession::new();
        match opening.open(&event.data()).await {
            Ok(()) => session.set(opening),
            Err(message) => {
                viewfinder_open.set(false);
                on_error.call(message);
            }
        }

        starting.set(false);
    };

    let take_photo = move |_| {
        // Bound to a local first so the read guard is released before
        // `close_viewfinder` asks for the same signal mutably.
        let captured = session.read().capture();

        match captured {
            Ok(photo) => {
                close_viewfinder();
                on_photo.call(photo);
            }
            Err(message) => on_error.call(message),
        }
    };

    let choose_file = move |event: Event<FormData>| async move {
        let Some(file) = event.files().into_iter().next() else {
            return;
        };

        if file.size() > MAX_PHOTO_BYTES {
            // One decimal place: whole megabytes round a 1.2 MB photo down to
            // the same "1 MB" as the limit it just exceeded.
            on_error.call(format!(
                "That photo is {:.1} MB; the limit is {:.1} MB",
                file.size() as f64 / 1_048_576.0,
                MAX_PHOTO_BYTES as f64 / 1_048_576.0
            ));
            return;
        }

        match file.read_bytes().await {
            Ok(bytes) => {
                let mime = file
                    .content_type()
                    .unwrap_or_else(|| "image/jpeg".to_string());
                on_photo.call((mime, STANDARD.encode(&bytes)));
            }
            Err(err) => on_error.call(format!("Could not read that photo: {err}")),
        }
    };

    rsx! {
        div { class: "photo-picker",
            if camera_available() {
                button {
                    disabled: viewfinder_open(),
                    onclick: move |_| viewfinder_open.set(true),
                    "Take photo"
                }
            }

            input {
                r#type: "file",
                // Narrows the picker to images, and on a phone offers the
                // camera alongside the library. `capture` asks for the rear
                // lens; browsers that do not understand it fall back to the
                // ordinary picker. This route stays available whether or not
                // the in-app viewfinder does.
                accept: "image/*",
                capture: "environment",
                onchange: choose_file,
            }
        }

        if viewfinder_open() {
            div { class: "viewfinder",
                video {
                    class: "viewfinder-preview",
                    autoplay: true,
                    muted: true,
                    // Without this iOS plays the stream in its own full-screen
                    // player instead of in the page.
                    playsinline: true,
                    onmounted: start_camera,
                }

                div { class: "viewfinder-controls",
                    button {
                        onclick: move |_| close_viewfinder(),
                        "Cancel"
                    }
                    button {
                        class: "primary",
                        disabled: starting(),
                        onclick: take_photo,
                        if starting() { "Starting…" } else { "Capture" }
                    }
                }
            }
        }
    }
}
