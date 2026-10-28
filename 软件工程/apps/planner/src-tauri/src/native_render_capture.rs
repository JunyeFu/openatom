//! Native PNG capture of an already prepared product WebView.
//!
//! The trusted caller owns window selection, authorization, frame readiness and
//! lifetime. This module never navigates, shows, focuses or resizes the window.

use std::{cell::RefCell, fmt, rc::Rc};

use tauri::{WebviewWindow, webview::PlatformWebview};
use tokio::sync::oneshot;
use webview2_com::{
    CapturePreviewCompletedHandler,
    Microsoft::Web::WebView2::Win32::COREWEBVIEW2_CAPTURE_PREVIEW_IMAGE_FORMAT_PNG,
};
use windows::Win32::{
    Foundation::HGLOBAL,
    System::Com::{
        IStream, STATFLAG_NONAME, STATSTG, STREAM_SEEK_SET,
        StructuredStorage::CreateStreamOnHGlobal,
    },
};

/// An owned error that can leave the WebView's STA without a COM interface.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CaptureError {
    pub stage: &'static str,
    pub message: String,
    pub hresult: Option<i32>,
}

impl CaptureError {
    fn new(stage: &'static str, message: impl Into<String>) -> Self {
        Self {
            stage,
            message: message.into(),
            hresult: None,
        }
    }

    fn com(stage: &'static str, error: windows::core::Error) -> Self {
        Self {
            stage,
            message: error.to_string(),
            hresult: Some(error.code().0),
        }
    }
}

impl fmt::Display for CaptureError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}: {}", self.stage, self.message)
    }
}

impl std::error::Error for CaptureError {}

type CaptureResult = Result<Vec<u8>, CaptureError>;
// Constructed only inside with_webview; neither this Rc nor an IStream crosses
// threads. The shared slot covers both immediate and asynchronous API failure.
type CompletionSender = Rc<RefCell<Option<oneshot::Sender<CaptureResult>>>>;

/// Capture the actual contents of a trusted, ready product WebView as PNG.
///
/// `with_webview` enters the original UI STA. CapturePreview and its completion
/// callback keep all COM objects there; only owned bytes/errors cross the channel.
/// Await this future without holding a Host mutex. Dropping it does not cancel
/// WebView2's capture; the caller must keep its task window alive until completion.
pub async fn capture_png(window: WebviewWindow) -> CaptureResult {
    let (sender, receiver) = oneshot::channel();
    window
        .with_webview(move |webview| {
            let completion = Rc::new(RefCell::new(Some(sender)));
            if let Err(error) = start_capture(webview, completion.clone()) {
                complete(&completion, Err(error));
            }
        })
        .map_err(|error| CaptureError::new("schedule_ui", error.to_string()))?;

    receiver.await.map_err(|_| {
        CaptureError::new(
            "completion_dropped",
            "WebView2 capture ended without a completion result",
        )
    })?
}

fn start_capture(
    webview: PlatformWebview,
    completion: CompletionSender,
) -> Result<(), CaptureError> {
    // SAFETY: Tauri invokes this function on the WebView's UI STA. The stream is
    // allocated here and is released here or by the callback on that same STA.
    let core = unsafe { webview.controller().CoreWebView2() }
        .map_err(|error| CaptureError::com("get_webview", error))?;
    let stream = unsafe { CreateStreamOnHGlobal(HGLOBAL::default(), true) }
        .map_err(|error| CaptureError::com("create_stream", error))?;
    let callback_stream = stream.clone();
    let callback = CapturePreviewCompletedHandler::create(Box::new(move |result| {
        let png = result
            .map_err(|error| CaptureError::com("capture_complete", error))
            .and_then(|()| read_png(&callback_stream));
        complete(&completion, png);
        Ok(())
    }));
    unsafe {
        core.CapturePreview(
            COREWEBVIEW2_CAPTURE_PREVIEW_IMAGE_FORMAT_PNG,
            &stream,
            &callback,
        )
    }
    .map_err(|error| CaptureError::com("capture_start", error))
}

fn complete(completion: &CompletionSender, result: CaptureResult) {
    if let Some(sender) = completion.borrow_mut().take() {
        // The receiver may have been dropped when its task was cancelled. There
        // is then no consumer; releasing the callback still releases its stream.
        let _ = sender.send(result);
    }
}

fn read_png(stream: &IStream) -> CaptureResult {
    let mut stat = STATSTG::default();
    // SAFETY: Called only in the STA completion callback with its live stream.
    // NONAME avoids allocating a COM-owned name that the caller would free.
    unsafe { stream.Stat(&mut stat, STATFLAG_NONAME) }
        .map_err(|error| CaptureError::com("stream_stat", error))?;
    let length = usize::try_from(stat.cbSize)
        .map_err(|_| CaptureError::new("stream_size", "PNG stream cannot fit in address space"))?;
    if length == 0 {
        return Err(CaptureError::new(
            "png_data",
            "CapturePreview returned an empty stream",
        ));
    }
    unsafe { stream.Seek(0, STREAM_SEEK_SET, None) }
        .map_err(|error| CaptureError::com("stream_seek", error))?;

    let mut bytes = Vec::new();
    bytes
        .try_reserve_exact(length)
        .map_err(|error| CaptureError::new("stream_size", error.to_string()))?;
    bytes.resize(length, 0);
    let mut offset = 0;
    while offset < length {
        let count = (length - offset).min(u32::MAX as usize) as u32;
        let mut read = 0;
        // SAFETY: The writable slice contains at least count bytes. IStream
        // returns its actual byte count; short/failed reads never publish PNG.
        unsafe {
            stream
                .Read(bytes[offset..].as_mut_ptr().cast(), count, Some(&mut read))
                .ok()
        }
        .map_err(|error| CaptureError::com("stream_read", error))?;
        if read == 0 || read > count {
            return Err(CaptureError::new(
                "stream_read",
                "PNG stream ended before its reported size",
            ));
        }
        offset += read as usize;
    }
    if !bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        return Err(CaptureError::new(
            "png_data",
            "CapturePreview did not return PNG bytes",
        ));
    }
    Ok(bytes)
}
