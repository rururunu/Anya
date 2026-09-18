//! Windows Office automation — Word/Excel/PowerPoint context + COM tools.

pub mod com;
pub mod context;
pub mod debug;
pub mod excel;
pub mod powerpoint;
pub mod tools;
pub mod word;
pub mod worker;

pub use context::{
    collect_office_context, collect_office_context_for_process, enrich_request_context,
    office_app_available, OfficeContext,
};
pub use tools::register_tools;
