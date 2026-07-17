use canvas_core::video::ValidatedUpstreamResource;
use time::OffsetDateTime;

use crate::{
    id::opaque_id,
    session::{SessionId, TrackHandle, TrackParent, TrackRecord},
};

opaque_id!(DownloadTicketId);

pub struct TicketRequest {
    pub session_id: SessionId,
    pub parent: TrackParent,
    pub track: TrackHandle,
    pub record: TrackRecord,
}

pub struct DownloadTicket {
    id: DownloadTicketId,
    session_id: SessionId,
    parent: TrackParent,
    track: TrackHandle,
    resource: ValidatedUpstreamResource,
    suggested_filename: String,
    created_at: OffsetDateTime,
    expires_at: OffsetDateTime,
}

impl DownloadTicket {
    pub(super) fn new(
        id: DownloadTicketId,
        request: TicketRequest,
        created_at: OffsetDateTime,
        expires_at: OffsetDateTime,
    ) -> Self {
        Self {
            id,
            session_id: request.session_id,
            parent: request.parent,
            track: request.track,
            resource: request.record.resource,
            suggested_filename: request.record.suggested_filename,
            created_at,
            expires_at,
        }
    }

    pub fn id(&self) -> &DownloadTicketId {
        &self.id
    }
    pub fn resource(&self) -> &ValidatedUpstreamResource {
        &self.resource
    }
    pub fn suggested_filename(&self) -> &str {
        &self.suggested_filename
    }
    pub fn expires_at(&self) -> OffsetDateTime {
        self.expires_at
    }
    pub fn created_at(&self) -> OffsetDateTime {
        self.created_at
    }
    pub(crate) fn session_id(&self) -> &SessionId {
        &self.session_id
    }
    pub(crate) fn parent(&self) -> &TrackParent {
        &self.parent
    }
    pub(crate) fn track(&self) -> &TrackHandle {
        &self.track
    }
}

impl std::fmt::Debug for DownloadTicket {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("DownloadTicket(<redacted>)")
    }
}
