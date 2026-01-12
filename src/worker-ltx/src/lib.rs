pub mod errors;
pub mod work;

pub use errors::Error;

pub use work::{JobResult, handle_job, handle_result, next_job_in_queue};
