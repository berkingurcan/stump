use rocket::{
	http::Status,
	response::{self, Responder},
	serde::{json::Json, Serialize},
	tokio::sync::mpsc,
	Request,
};
use rocket_okapi::{
	gen::OpenApiGenerator, okapi::openapi3::Responses, response::OpenApiResponderInner,
	OpenApiError,
};
use rocket_session_store::SessionError;
use schemars::Map;
use specta::Type;
use stump_core::{
	event::InternalCoreTask,
	types::errors::{CoreError, ProcessFileError},
};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AuthError {
	#[error("Error during the authentication process")]
	BcryptError(#[from] bcrypt::BcryptError),
	#[error("Missing or malformed credentials")]
	BadCredentials,
	#[error("The Authorization header could no be parsed")]
	BadRequest,
	#[error("Unauthorized")]
	Unauthorized,
	#[error("Forbidden")]
	Forbidden,
	#[error("The session is not valid")]
	InvalidSession(#[from] SessionError),
}

#[derive(Serialize, Error, Debug, Type)]
// TODO: agh, naming is so hard. code vs kind, details vs data...
#[serde(tag = "code", content = "details")]
pub enum ApiError {
	#[error("{0}")]
	BadRequest(String),
	#[error("{0}")]
	NotFound(String),
	#[error("{0}")]
	InternalServerError(String),
	#[error("{0}")]
	Unauthorized(String),
	#[error("{0}")]
	Forbidden(String),
	#[error("{0}")]
	NotImplemented(String),
	#[error("{0}")]
	ServiceUnavailable(String),
	#[error("{0}")]
	BadGateway(String),
	#[error("{0}")]
	Unknown(String),
	#[error("{0}")]
	Redirect(String),
}

impl From<CoreError> for ApiError {
	fn from(err: CoreError) -> Self {
		match err {
			CoreError::InternalError(err) => ApiError::InternalServerError(err),
			CoreError::IoError(err) => ApiError::InternalServerError(err.to_string()),
			CoreError::MigrationError(err) => ApiError::InternalServerError(err),
			CoreError::QueryError(err) => ApiError::InternalServerError(err.to_string()),
			CoreError::Unknown(err) => ApiError::InternalServerError(err),
			CoreError::Utf8ConversionError(err) => {
				ApiError::InternalServerError(err.to_string())
			},
			CoreError::XmlWriteError(err) => {
				ApiError::InternalServerError(err.to_string())
			},
			_ => ApiError::InternalServerError(err.to_string()),
		}
	}
}

impl OpenApiResponderInner for ApiError {
	fn responses(_generator: &mut OpenApiGenerator) -> Result<Responses, OpenApiError> {
		use rocket_okapi::okapi::openapi3::{RefOr, Response as OpenApiReponse};

		let mut responses = Map::new();
		responses.insert(
            "400".to_string(),
            RefOr::Object(OpenApiReponse {
                description: "\
                # [400 Bad Request](https://developer.mozilla.org/en-US/docs/Web/HTTP/Status/400)\n\
                The request given is wrongly formatted or data asked could not be fulfilled. \
                "
                .to_string(),
                ..Default::default()
            }),
        );
		responses.insert(
            "401".to_string(),
            RefOr::Object(OpenApiReponse {
                description: "\
                # [401 Unauthorized](https://developer.mozilla.org/en-US/docs/Web/HTTP/Status/401)\n\
                You do not have authorization to make the request made. You must authenticate first. \
                "
                .to_string(),
                ..Default::default()
            }),
        );
		responses.insert(
            "403".to_string(),
            RefOr::Object(OpenApiReponse {
                description: "\
                # [403 Forbidden](https://developer.mozilla.org/en-US/docs/Web/HTTP/Status/403)\n\
                You do not have permission to make the request made. \
                "
                .to_string(),
                ..Default::default()
            }),
        );
		responses.insert(
            "404".to_string(),
            RefOr::Object(OpenApiReponse {
                description: "\
                # [404 Not Found](https://developer.mozilla.org/en-US/docs/Web/HTTP/Status/404)\n\
                This response is given when you request a page that does not exists.\
                "
                .to_string(),
                ..Default::default()
            }),
        );
		responses.insert(
            "500".to_string(),
            RefOr::Object(OpenApiReponse {
                description: "\
                # [500 Internal Server Error](https://developer.mozilla.org/en-US/docs/Web/HTTP/Status/500)\n\
                This response is given when something went wrong on the server. \
                ".to_string(),
                ..Default::default()
            }),
        );
		Ok(Responses {
			responses,
			..Default::default()
		})
	}
}

impl From<ApiError> for Status {
	fn from(error: ApiError) -> Status {
		match error {
			ApiError::BadRequest(_) => Status::BadRequest,
			ApiError::NotFound(_) => Status::NotFound,
			ApiError::InternalServerError(_) => Status::InternalServerError,
			ApiError::Unauthorized(_) => Status::Unauthorized,
			ApiError::Forbidden(_) => Status::Forbidden,
			ApiError::NotImplemented(_) => Status::NotImplemented,
			ApiError::ServiceUnavailable(_) => Status::ServiceUnavailable,
			ApiError::BadGateway(_) => Status::BadGateway,
			ApiError::Unknown(_) => Status::InternalServerError,
			// TODO: is this the right status? 308?
			ApiError::Redirect(_) => Status::PermanentRedirect,
		}
	}
}

impl From<&ApiError> for Status {
	fn from(error: &ApiError) -> Status {
		match error {
			ApiError::BadRequest(_) => Status::BadRequest,
			ApiError::NotFound(_) => Status::NotFound,
			ApiError::InternalServerError(_) => Status::InternalServerError,
			ApiError::Unauthorized(_) => Status::Unauthorized,
			ApiError::Forbidden(_) => Status::Forbidden,
			ApiError::NotImplemented(_) => Status::NotImplemented,
			ApiError::ServiceUnavailable(_) => Status::ServiceUnavailable,
			ApiError::BadGateway(_) => Status::BadGateway,
			ApiError::Unknown(_) => Status::InternalServerError,
			// TODO: is this the right status? 308?
			ApiError::Redirect(_) => Status::PermanentRedirect,
		}
	}
}

// TODO: look into how prisma returns record not found errors?
impl From<prisma_client_rust::queries::QueryError> for ApiError {
	fn from(error: prisma_client_rust::queries::QueryError) -> ApiError {
		ApiError::InternalServerError(error.to_string())
	}
}

impl From<prisma_client_rust::RelationNotFetchedError> for ApiError {
	fn from(e: prisma_client_rust::RelationNotFetchedError) -> Self {
		ApiError::InternalServerError(e.to_string())
	}
}

impl From<AuthError> for ApiError {
	fn from(error: AuthError) -> ApiError {
		match error {
			AuthError::BcryptError(_) => {
				ApiError::InternalServerError("Internal server error".to_string())
			},
			AuthError::BadCredentials => {
				ApiError::BadRequest("Missing or malformed credentials".to_string())
			},
			AuthError::BadRequest => ApiError::BadRequest(
				"The Authorization header could no be parsed".to_string(),
			),
			AuthError::Unauthorized => ApiError::Unauthorized("Unauthorized".to_string()),
			AuthError::Forbidden => ApiError::Forbidden("Forbidden".to_string()),
			AuthError::InvalidSession(_) => {
				ApiError::InternalServerError("Internal server error".to_string())
			},
		}
	}
}

impl From<SessionError> for ApiError {
	fn from(error: SessionError) -> ApiError {
		ApiError::InternalServerError(error.to_string())
	}
}

impl From<ProcessFileError> for ApiError {
	fn from(error: ProcessFileError) -> ApiError {
		ApiError::InternalServerError(error.to_string())
	}
}

// impl From<anyhow::Error> for ApiError {
// 	fn from(error: anyhow::Error) -> ApiError {
// 		ApiError::InternalServerError(error.to_string())
// 	}
// }

impl From<String> for ApiError {
	fn from(msg: String) -> ApiError {
		ApiError::InternalServerError(msg)
	}
}

impl From<&str> for ApiError {
	fn from(msg: &str) -> ApiError {
		ApiError::InternalServerError(msg.to_string())
	}
}

impl From<std::num::TryFromIntError> for ApiError {
	fn from(error: std::num::TryFromIntError) -> ApiError {
		ApiError::InternalServerError(error.to_string())
	}
}

impl From<std::io::Error> for ApiError {
	fn from(error: std::io::Error) -> ApiError {
		ApiError::InternalServerError(error.to_string())
	}
}

impl From<bcrypt::BcryptError> for ApiError {
	fn from(error: bcrypt::BcryptError) -> ApiError {
		ApiError::InternalServerError(error.to_string())
	}
}

impl From<mpsc::error::SendError<InternalCoreTask>> for ApiError {
	fn from(err: mpsc::error::SendError<InternalCoreTask>) -> Self {
		ApiError::InternalServerError(err.to_string())
	}
}

impl<'r> Responder<'r, 'static> for ApiError {
	fn respond_to(self, req: &'r Request<'_>) -> response::Result<'static> {
		let status = Status::from(&self);
		let body = Json(self);

		let mut response = body.respond_to(req)?;

		response.set_status(status);

		Ok(response)
	}
}
