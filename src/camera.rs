//! The device camera, behind a surface that compiles everywhere.
//!
//! Every browser type stays inside this module. That lets the views offer a
//! viewfinder without themselves depending on a DOM, so the same view code
//! still compiles into the native server binary — where the camera simply
//! reports itself unavailable and the file picker carries the feature alone.

use dioxus::prelude::*;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::{JsCast, JsValue};

/// Quality used when encoding a captured frame.
///
/// A frame arrives from the camera as raw pixels, so unlike a file chosen from
/// the library there is no original encoding to preserve — it has to be
/// compressed on the way out regardless. 0.92 keeps the encoder well clear of
/// being what limits the picture. The frame is written at the resolution the
/// camera produced; nothing is scaled.
#[cfg(target_arch = "wasm32")]
const CAPTURE_QUALITY: f64 = 0.92;

#[cfg(target_arch = "wasm32")]
const CAPTURE_MIME: &str = "image/jpeg";

/// Resolution asked of the camera, in pixels along each edge.
///
/// Requested as `ideal`, which asks for the closest mode a device can manage,
/// rather than `exact`, which is a demand it can fail outright. Deliberately
/// larger than any current sensor so the browser settles on the best mode it
/// has instead of a default that is usually 640×480.
#[cfg(target_arch = "wasm32")]
const IDEAL_EDGE_PIXELS: f64 = 4096.0;

/// Whether a live viewfinder can be opened here.
///
/// False on the server, and false in a browser that does not expose
/// `getUserMedia` — which over a plain-HTTP LAN address is the ordinary case,
/// since the API is only present in a secure context. This decides whether the
/// camera is offered *in addition to* the file picker, never whether the file
/// picker itself is offered.
pub fn is_supported() -> bool {
    #[cfg(target_arch = "wasm32")]
    {
        web_sys::window().is_some_and(|window| window.navigator().media_devices().is_ok())
    }

    #[cfg(not(target_arch = "wasm32"))]
    false
}

/// A camera held open for as long as the viewfinder is on screen.
///
/// Present on every target so a view can store one unconditionally; off the web
/// it holds nothing and every operation reports that there is no camera.
#[derive(Default)]
pub struct CameraSession {
    #[cfg(target_arch = "wasm32")]
    stream: Option<web_sys::MediaStream>,
    #[cfg(target_arch = "wasm32")]
    video: Option<web_sys::HtmlVideoElement>,
}

impl CameraSession {
    pub fn new() -> Self {
        Self::default()
    }

    /// Starts the rear camera and shows it in `element`, which must be the
    /// `video` node the viewfinder has just mounted.
    pub async fn open(&mut self, element: &MountedData) -> Result<(), String> {
        #[cfg(target_arch = "wasm32")]
        {
            let video = element
                .downcast::<web_sys::Element>()
                .and_then(|element| element.dyn_ref::<web_sys::HtmlVideoElement>())
                .ok_or("The viewfinder is missing from the page")?
                .clone();

            let window = web_sys::window().ok_or("There is no browser window here")?;
            let devices = window.navigator().media_devices().map_err(|_| {
                "This browser will not hand over the camera. Over plain HTTP that is expected \
                 — cameras are only offered on localhost or HTTPS."
                    .to_string()
            })?;

            // `environment` asks for the rear lens, which is the one pointed at
            // the food. Both are preferences: a device with only one camera
            // still opens it rather than refusing.
            let video_constraints = js_sys::Object::new();
            set_property(&video_constraints, "facingMode", &"environment".into());
            set_property(&video_constraints, "width", &ideal(IDEAL_EDGE_PIXELS));
            set_property(&video_constraints, "height", &ideal(IDEAL_EDGE_PIXELS));

            let constraints = web_sys::MediaStreamConstraints::new();
            constraints.set_video(&video_constraints);
            // Asking for audio would put a microphone in the permission prompt
            // that this feature has no use for.
            constraints.set_audio_bool(false);

            let request = devices
                .get_user_media_with_constraints(&constraints)
                .map_err(describe_camera_error)?;

            let stream: web_sys::MediaStream = wasm_bindgen_futures::JsFuture::from(request)
                .await
                .map_err(describe_camera_error)?
                .dyn_into()
                .map_err(|_| "The camera returned something that is not a video stream".to_string())?;

            // Muted as a property and not only as an attribute: iOS checks the
            // property before it will play a stream inline, and without it the
            // preview is taken over by the system's full-screen player.
            video.set_muted(true);
            video.set_src_object(Some(&stream));

            // Playback is started but not awaited. A browser that defers it has
            // not failed — the stream is attached either way, and the capture
            // path checks for a frame before it uses one.
            let _ = video.play();

            self.stream = Some(stream);
            self.video = Some(video);

            Ok(())
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            let _ = element;
            Err("There is no camera on this platform".to_string())
        }
    }

    /// Encodes the frame currently on screen as `(mime, base64)`, ready to be
    /// handed to `set_recipe_photo` unchanged.
    pub fn capture(&self) -> Result<(String, String), String> {
        #[cfg(target_arch = "wasm32")]
        {
            let video = self.video.as_ref().ok_or("The camera is not open")?;

            // Zero until the first frame has arrived and the browser knows the
            // stream's dimensions. Drawing then would save a blank rectangle,
            // which is worse than asking for another try.
            let (width, height) = (video.video_width(), video.video_height());
            if width == 0 || height == 0 {
                return Err("The camera is still starting up. Try again in a moment.".to_string());
            }

            let document = web_sys::window()
                .and_then(|window| window.document())
                .ok_or("There is no document to draw into")?;

            // Sized to the frame exactly, so the picture is kept at whatever
            // resolution the camera produced rather than being resampled.
            let canvas: web_sys::HtmlCanvasElement = document
                .create_element("canvas")
                .map_err(|_| "Could not create a canvas".to_string())?
                .dyn_into()
                .map_err(|_| "Could not create a canvas".to_string())?;
            canvas.set_width(width);
            canvas.set_height(height);

            let context: web_sys::CanvasRenderingContext2d = canvas
                .get_context("2d")
                .map_err(|_| "Could not get a drawing context".to_string())?
                .ok_or("This browser offers no 2D canvas")?
                .dyn_into()
                .map_err(|_| "Could not get a drawing context".to_string())?;

            context
                .draw_image_with_html_video_element(video, 0.0, 0.0)
                .map_err(|_| "The frame could not be copied off the camera".to_string())?;

            // Encoding runs on the main thread and a full-resolution frame is
            // several megabytes of base64, so the page pauses briefly here.
            // That is the cost of storing the photo unscaled.
            let data_url = canvas
                .to_data_url_with_type_and_encoder_options(
                    CAPTURE_MIME,
                    &JsValue::from_f64(CAPTURE_QUALITY),
                )
                .map_err(|_| "The frame could not be encoded".to_string())?;

            // Comes back as `data:image/jpeg;base64,<payload>`; callers want the
            // payload alone, since that is what crosses the wire.
            let payload = data_url
                .split_once(";base64,")
                .ok_or("The frame came back in an unexpected format")?
                .1
                .to_string();

            Ok((CAPTURE_MIME.to_string(), payload))
        }

        #[cfg(not(target_arch = "wasm32"))]
        Err("There is no camera on this platform".to_string())
    }

    /// Releases the camera. This is what turns the indicator light off, so it
    /// runs whenever the viewfinder goes away — by capture, by cancel, or by
    /// the component being torn down.
    pub fn close(&mut self) {
        #[cfg(target_arch = "wasm32")]
        {
            // Detached first, so the preview is never left showing a stream
            // whose tracks have already stopped.
            if let Some(video) = self.video.take() {
                video.set_src_object(None);
            }

            // Each track has to be stopped on its own. Dropping the stream is
            // not enough: the camera, and its light, stay on until every track
            // that came from it ends.
            if let Some(stream) = self.stream.take() {
                for track in stream.get_tracks().iter() {
                    if let Some(track) = track.dyn_ref::<web_sys::MediaStreamTrack>() {
                        track.stop();
                    }
                }
            }
        }
    }
}

/// Stops the camera even if the viewfinder is torn down without passing through
/// any of the paths that would normally close it.
impl Drop for CameraSession {
    fn drop(&mut self) {
        self.close();
    }
}

/// Sets one field on a plain JS options object.
///
/// The result is discarded because it can only fail on an object that rejects
/// the write, and these are freshly made and extensible.
#[cfg(target_arch = "wasm32")]
fn set_property(target: &js_sys::Object, key: &str, value: &JsValue) {
    let _ = js_sys::Reflect::set(target, &JsValue::from_str(key), value);
}

/// Builds the `{ ideal: n }` form that a media constraint takes.
#[cfg(target_arch = "wasm32")]
fn ideal(pixels: f64) -> JsValue {
    let constraint = js_sys::Object::new();
    set_property(&constraint, "ideal", &JsValue::from_f64(pixels));
    constraint.into()
}

/// Explains a refused camera in terms of what to do about it.
///
/// These arrive as a `DOMException` whose `name` is the part that distinguishes
/// them; the message alongside it is usually empty or too generic to act on.
#[cfg(target_arch = "wasm32")]
fn describe_camera_error(error: JsValue) -> String {
    let Some(exception) = error.dyn_ref::<web_sys::DomException>() else {
        return "The camera could not be opened".to_string();
    };

    match exception.name().as_str() {
        "NotAllowedError" | "SecurityError" => {
            "Permission to use the camera was refused. Allow it for this site in the browser's \
             settings, then try again."
                .to_string()
        }
        "NotFoundError" | "OverconstrainedError" => {
            "No camera was found on this device.".to_string()
        }
        "NotReadableError" => {
            "The camera is already in use by another app.".to_string()
        }
        other => format!("The camera could not be opened ({other})."),
    }
}
