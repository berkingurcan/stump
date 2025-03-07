pub mod library_scan;
pub mod pool;
pub mod runner;

use std::fmt::Debug;

use rocket_okapi::JsonSchema;
use serde::{Deserialize, Serialize};
use specta::Type;

use crate::{
	config::context::Ctx,
	event::ClientEvent,
	prisma::{self},
	types::errors::ApiError,
};

#[derive(Clone, Serialize, Deserialize, Debug, JsonSchema, Type)]
pub enum JobStatus {
	#[serde(rename = "RUNNING")]
	Running,
	#[serde(rename = "QUEUED")]
	Queued,
	#[serde(rename = "COMPLETED")]
	Completed,
	#[serde(rename = "CANCELLED")]
	Cancelled,
	#[serde(rename = "FAILED")]
	Failed,
}

impl std::fmt::Display for JobStatus {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			JobStatus::Running => write!(f, "RUNNING"),
			JobStatus::Queued => write!(f, "QUEUED"),
			JobStatus::Completed => write!(f, "COMPLETED"),
			JobStatus::Cancelled => write!(f, "CANCELLED"),
			JobStatus::Failed => write!(f, "FAILED"),
		}
	}
}

impl From<&str> for JobStatus {
	fn from(s: &str) -> Self {
		match s {
			"RUNNING" => JobStatus::Running,
			"QUEUED" => JobStatus::Queued,
			"COMPLETED" => JobStatus::Completed,
			"CANCELLED" => JobStatus::Cancelled,
			"FAILED" => JobStatus::Failed,
			_ => unreachable!(),
		}
	}
}

#[derive(Clone, Serialize, Deserialize, Debug, Type)]
#[serde(rename_all = "camelCase")]
pub struct JobUpdate {
	pub runner_id: String,
	// TODO: don't use option. This is a temporary workaround for the Arc issue with
	// batch scan mode.
	pub current_task: Option<u64>,
	pub task_count: u64,
	// TODO: change this to data: Option<T: Serialize> or something...
	pub message: Option<String>,
	pub status: Option<JobStatus>,
}

#[derive(Clone, Serialize, Deserialize, Debug, JsonSchema, Type)]
#[serde(rename_all = "camelCase")]
pub struct JobReport {
	/// This will actually refer to the job runner id
	pub id: Option<String>,
	/// The kind of log, e.g. LibraryScanJob
	pub kind: String,
	/// The extra details of the job, e.g. "/Users/oromei/Documents/Stump/MainLibrary"
	pub details: Option<String>,
	/// The status of the job (i.e. COMPLETED, FAILED, CANCELLED). Running jobs are not persisted to DB.
	status: JobStatus,
	/// The total number of tasks
	task_count: Option<i32>,
	/// The total number of tasks completed (i.e. without error/failure)
	completed_task_count: Option<i32>,
	/// The time (in seconds) to complete the job
	seconds_elapsed: Option<u64>,
	/// The datetime stamp of when the job completed
	completed_at: Option<String>,
}

impl From<prisma::job::Data> for JobReport {
	fn from(data: prisma::job::Data) -> Self {
		JobReport {
			id: Some(data.id),
			kind: data.kind,
			details: data.details,
			status: JobStatus::from(data.status.as_str()),
			task_count: Some(data.task_count),
			completed_task_count: Some(data.completed_task_count),
			seconds_elapsed: Some(data.seconds_elapsed as u64),
			completed_at: Some(data.completed_at.to_string()),
		}
	}
}

impl From<&Box<dyn Job>> for JobReport {
	fn from(job: &Box<dyn Job>) -> Self {
		Self {
			id: None,
			kind: job.kind().to_string(),
			details: job.details().map(|d| d.clone().to_string()),
			status: JobStatus::Queued,

			task_count: None,
			completed_task_count: None,
			seconds_elapsed: None,
			completed_at: None,
		}
	}
}

#[async_trait::async_trait]
pub trait Job: Send + Sync {
	fn kind(&self) -> &'static str;
	fn details(&self) -> Option<Box<&str>>;

	async fn run(&self, runner_id: String, ctx: Ctx) -> Result<(), ApiError>;
}

pub async fn persist_new_job(
	ctx: &Ctx,
	id: String,
	job: &Box<dyn Job>,
) -> Result<crate::prisma::job::Data, ApiError> {
	use crate::prisma::job;

	let db = ctx.get_db();

	Ok(db
		.job()
		.create(
			id,
			job.kind().to_string(),
			vec![
				job::details::set(job.details().map(|d| d.clone().to_string())),
				// job::task_count::set(task_count.try_into()?),
			],
		)
		.exec()
		.await?)
}

pub async fn persist_job_start(
	ctx: &Ctx,
	id: String,
	task_count: u64,
) -> Result<crate::prisma::job::Data, ApiError> {
	use crate::prisma::job;

	let db = ctx.get_db();

	let job = db
		.job()
		.update(
			job::id::equals(id.clone()),
			vec![
				job::task_count::set(task_count.try_into()?),
				job::status::set(JobStatus::Running.to_string()),
			],
		)
		.exec()
		.await?;

	ctx.emit_client_event(ClientEvent::job_started(
		id.clone(),
		1,
		task_count,
		Some(format!("Job {} started.", id)),
	));

	Ok(job)
}

pub async fn persist_job_end(
	ctx: &Ctx,
	id: String,
	completed_task_count: u64,
	elapsed_seconds: u64,
) -> Result<crate::prisma::job::Data, ApiError> {
	use crate::prisma::job;

	let db = ctx.get_db();

	let job = db
		.job()
		.update(
			job::id::equals(id.clone()),
			vec![
				job::completed_task_count::set(completed_task_count.try_into()?),
				job::seconds_elapsed::set(elapsed_seconds.try_into()?),
				job::status::set(JobStatus::Completed.to_string()),
			],
		)
		.exec()
		.await?;

	Ok(job)
}
