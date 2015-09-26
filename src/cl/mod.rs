pub mod device;
pub mod program;
pub mod context;
pub mod platform;
pub mod command_queue;
pub mod kernel;
pub mod buffer;

use opencl::cl::*;
use std::fmt::{Display, Formatter, Result};

#[derive(Debug)]
pub struct OpenClError {
	human_error: String,
	error_code: Option<i32>,
}

impl OpenClError {
	pub fn new(human_error: String, error_code: cl_int) -> OpenClError {
		OpenClError {
			human_error: human_error,
			error_code: Some(error_code as i32),
		}
	}

	pub fn from_string(human_error: String) -> OpenClError {
		OpenClError {
			human_error: human_error,
			error_code: None,
		}
	}
}

impl Display for OpenClError {
	fn fmt(&self, f: &mut Formatter) -> Result {
		match self.error_code {
			None => write!(f, "{}", self.human_error),
			Some(err_code) => write!(f, "(HR: {}, CL err code: {})", self.human_error, err_code as i64)
		}
    }	
}