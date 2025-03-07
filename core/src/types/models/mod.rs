pub mod epub;
pub mod library;
pub mod list_directory;
pub mod log;
pub mod media;
pub mod read_progress;
pub mod series;
pub mod tag;
pub mod user;

use rocket_okapi::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::prisma;

use self::user::UserPreferences;

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct AuthenticatedUser {
	pub id: String,
	pub username: String,
	pub role: String,
	pub user_preferences: UserPreferences,
}

impl Into<AuthenticatedUser> for prisma::user::Data {
	fn into(self) -> AuthenticatedUser {
		let user_preferences = match self
			.user_preferences()
			.expect("Failed to load user preferences")
		{
			Some(preferences) => preferences.to_owned(),
			None => unreachable!(
				"User does not have preferences. This should not be reachable."
			),
		};

		AuthenticatedUser {
			id: self.id.clone(),
			username: self.username.clone(),
			role: self.role.clone(),
			user_preferences: user_preferences.into(),
		}
	}
}

#[derive(Debug)]
pub struct DecodedCredentials {
	pub username: String,
	pub password: String,
}

#[derive(Deserialize, JsonSchema)]
pub struct LoginRequest {
	pub username: String,
	pub password: String,
}
