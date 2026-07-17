mod handles;
mod model;
mod resources;
mod store;

pub use handles::{
    CourseHandle, CourseRecord, CourseView, HandleWindow, TrackHandle, TrackParent, TrackRecord,
    TrackRegistration, TrackView, VideoHandle, VideoRecord, VideoView,
};
pub use model::{UserSession, UserSessionOptions};
pub use resources::SessionResources;
pub use store::{SessionLookupError, SessionStore};

use crate::id::opaque_id;

opaque_id!(SessionId);
