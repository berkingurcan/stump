use std::{fmt, str::FromStr};

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use specta::Type;

#[derive(Serialize, Deserialize, Type)]
pub enum UserRole {
	#[serde(rename = "SERVER_OWNER")]
	ServerOwner,
	#[serde(rename = "MEMBER")]
	Member,
}

#[derive(Serialize, Deserialize, Type)]
pub enum LayoutMode {
	#[serde(rename = "GRID")]
	Grid,
	#[serde(rename = "LIST")]
	List,
}

#[derive(Debug, Deserialize, Serialize, JsonSchema, Type, Clone, Copy)]
pub enum FileStatus {
	#[serde(rename = "UNKNOWN")]
	Unknown,
	#[serde(rename = "READY")]
	Ready,
	#[serde(rename = "UNSUPPORTED")]
	Unsupported,
	#[serde(rename = "ERROR")]
	Error,
	#[serde(rename = "MISSING")]
	Missing,
}

impl fmt::Display for FileStatus {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		match self {
			FileStatus::Unknown => write!(f, "UNKNOWN"),
			FileStatus::Ready => write!(f, "READY"),
			FileStatus::Unsupported => write!(f, "UNSUPPORTED"),
			FileStatus::Error => write!(f, "ERROR"),
			FileStatus::Missing => write!(f, "MISSING"),
		}
	}
}

impl FromStr for FileStatus {
	type Err = ();

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		match s {
			"UNKNOWN" => Ok(FileStatus::Unknown),
			"READY" => Ok(FileStatus::Ready),
			"UNSUPPORTED" => Ok(FileStatus::Unsupported),
			"ERROR" => Ok(FileStatus::Error),
			"MISSING" => Ok(FileStatus::Missing),
			_ => Err(()),
		}
	}
}

impl Default for UserRole {
	fn default() -> Self {
		UserRole::Member
	}
}

impl Into<String> for UserRole {
	fn into(self) -> String {
		match self {
			UserRole::ServerOwner => "SERVER_OWNER".to_string(),
			UserRole::Member => "MEMBER".to_string(),
		}
	}
}
