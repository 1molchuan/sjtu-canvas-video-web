#![forbid(unsafe_code)]
//! Protocol-domain types and, after Phase 1 validation, the upstream protocol implementation.
//!
//! Phase 0 intentionally contains no live jAccount, Canvas, LTI, or video requests. Hard-coded
//! endpoints from the reference desktop application remain research evidence until validated.

pub mod canvas;
pub mod client;
pub mod constants;
pub mod error;
pub mod jaccount;
pub mod lti;
pub mod model;
pub mod redaction;
pub mod video;

pub use error::{CoreError, CoreErrorCode, ProtocolError};
pub use model::{CanvasCourse, CanvasVideo, UserIdentity, VideoTrack};
