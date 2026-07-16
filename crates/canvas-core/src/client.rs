//! Per-user upstream HTTP client construction will be implemented after protocol validation.
//!
//! The implementation must own an independent cookie jar for every authenticated website
//! session. No process-global client or token state is permitted.
