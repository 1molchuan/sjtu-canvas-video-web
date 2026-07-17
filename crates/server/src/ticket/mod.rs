mod model;
mod store;

pub use model::{DownloadTicket, DownloadTicketId, TicketRequest};
pub use store::{DownloadTicketStore, TicketLookupError, TicketStoreError};
