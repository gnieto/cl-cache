use cl::platform::Platform;
use regex::Regex;

pub struct ClRoot;

impl ClRoot {
	pub fn get_platform(pq: &PlatformQuery) -> Option<Platform> {
		match *pq {
			PlatformQuery::Default => Self::platform_from_index(0),
			PlatformQuery::Index(i) => Self::platform_from_index(i),
			PlatformQuery::Regexp(ref r) => Self::platform_from_regexp(r),
		}
	}

	fn platform_from_index(index: usize) -> Option<Platform> {
		let platforms = Self::get_platforms();

		Self::platform_from_idx(&platforms, index)
	}

	fn platform_from_regexp(regex: &Regex) -> Option<Platform> {
		let platforms = Self::get_platforms();

		let index = platforms.iter().position(|ref x| regex.is_match(&x.name()));

		match index {
			None => None,
			Some(_) => Some(platforms[0].clone())
		}
	}

	fn get_platforms() -> Vec<Platform> {
		Platform::all().unwrap_or(Vec::new())
	}

	fn platform_from_idx(platforms: &Vec<Platform>, index: usize) -> Option<Platform> {
		if platforms.len() > 0 {
			Some(platforms[index].clone())
		} else {
			None
		}		
	}
}

#[derive(Debug)]
pub enum PlatformQuery {
	Default,
	Index(usize),
	Regexp(Regex),
}