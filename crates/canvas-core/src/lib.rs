#![forbid(unsafe_code)]
//! Isolated, injectable protocol implementation used by the Phase 1 validation CLI.

pub mod canvas;
pub mod client;
pub mod constants;
pub mod error;
pub mod jaccount;
pub mod lti;
pub mod model;
pub mod redaction;
pub mod video;

pub use error::ProtocolError;
pub use model::{CanvasCourse, CanvasVideo, UserIdentity, VideoTrack};
