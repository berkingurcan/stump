use axum::{middleware::from_extractor, routing::get, Extension, Router};
use axum_sessions::extractors::ReadableSession;
use prisma_client_rust::{chrono, Direction};
use stump_core::{
	opds::{
		entry::OpdsEntry,
		feed::OpdsFeed,
		link::{OpdsLink, OpdsLinkRel, OpdsLinkType},
	},
	prisma::{media, read_progress},
};

use crate::{
	config::state::State,
	errors::ApiResult,
	middleware::auth::Auth,
	utils::{get_session_user, http::Xml},
};

pub(crate) fn mount() -> Router {
	Router::new()
		.nest(
			"/opds/v1.2",
			Router::new()
				.route("/catalog", get(catalog))
				.route("/keep-reading", get(keep_reading))
				.route("/libraries", get(libraries)),
		)
		.layer(from_extractor::<Auth>())
}

// TODO auth middleware....
async fn catalog() -> ApiResult<Xml> {
	let entries = vec![
		OpdsEntry::new(
			"keepReading".to_string(),
			chrono::Utc::now().into(),
			"Keep reading".to_string(),
			Some(String::from("Continue reading your in progress books")),
			None,
			Some(vec![OpdsLink {
				link_type: OpdsLinkType::Navigation,
				rel: OpdsLinkRel::Subsection,
				href: String::from("/opds/v1.2/keep-reading"),
			}]),
			None,
		),
		OpdsEntry::new(
			"allSeries".to_string(),
			chrono::Utc::now().into(),
			"All series".to_string(),
			Some(String::from("Browse by series")),
			None,
			Some(vec![OpdsLink {
				link_type: OpdsLinkType::Navigation,
				rel: OpdsLinkRel::Subsection,
				href: String::from("/opds/v1.2/series"),
			}]),
			None,
		),
		OpdsEntry::new(
			"latestSeries".to_string(),
			chrono::Utc::now().into(),
			"Latest series".to_string(),
			Some(String::from("Browse latest series")),
			None,
			Some(vec![OpdsLink {
				link_type: OpdsLinkType::Navigation,
				rel: OpdsLinkRel::Subsection,
				href: String::from("/opds/v1.2/series/latest"),
			}]),
			None,
		),
		OpdsEntry::new(
			"allLibraries".to_string(),
			chrono::Utc::now().into(),
			"All libraries".to_string(),
			Some(String::from("Browse by library")),
			None,
			Some(vec![OpdsLink {
				link_type: OpdsLinkType::Navigation,
				rel: OpdsLinkRel::Subsection,
				href: String::from("/opds/v1.2/libraries"),
			}]),
			None,
		),
		// OpdsEntry::new(
		// 	"allCollections".to_string(),
		// 	chrono::Utc::now().into(),
		// 	"All collections".to_string(),
		// 	Some(String::from("Browse by collection")),
		// 	None,
		// 	Some(vec![OpdsLink {
		// 		link_type: OpdsLinkType::Navigation,
		// 		rel: OpdsLinkRel::Subsection,
		// 		href: String::from("/opds/v1.2/collections"),
		// 	}]),
		// 	None,
		// ),
		// OpdsEntry::new(
		// 	"allReadLists".to_string(),
		// 	chrono::Utc::now().into(),
		// 	"All read lists".to_string(),
		// 	Some(String::from("Browse by read list")),
		// 	None,
		// 	Some(vec![OpdsLink {
		// 		link_type: OpdsLinkType::Navigation,
		// 		rel: OpdsLinkRel::Subsection,
		// 		href: String::from("/opds/v1.2/readlists"),
		// 	}]),
		// 	None,
		// ),
		// TODO: more?
		// TODO: get user stored searches, so they don't have to redo them over and over?
		// e.g. /opds/v1.2/series?search={searchTerms}, /opds/v1.2/libraries?search={searchTerms}, etc.
	];

	let links = vec![
		OpdsLink {
			link_type: OpdsLinkType::Navigation,
			rel: OpdsLinkRel::ItSelf,
			href: String::from("/opds/v1.2/catalog"),
		},
		OpdsLink {
			link_type: OpdsLinkType::Navigation,
			rel: OpdsLinkRel::Start,
			href: String::from("/opds/v1.2/catalog"),
		},
		// OpdsLink {
		// 	link_type: OpdsLinkType::Search,
		// 	rel: OpdsLinkRel::Search,
		// 	href: String::from("/opds/v1.2/search"),
		// },
	];

	let feed = OpdsFeed::new(
		"root".to_string(),
		"Stump OPDS catalog".to_string(),
		Some(links),
		entries,
	);

	Ok(Xml(feed.build()?))
}

async fn keep_reading(Extension(ctx): State, session: ReadableSession) -> ApiResult<Xml> {
	let db = ctx.get_db();

	let user_id = get_session_user(&session)?.id;

	// FIXME: not sure how to go about fixing this query. I kind of need to load all the
	// media, so I know which ones are 'completed'. I think the solution here is to just create
	// a new field on read_progress called 'completed'.
	// Lol just noticed I already said that below. Guess I'll do that before next PR.
	let media = db
		.media()
		.find_many(vec![media::read_progresses::some(vec![
			read_progress::user_id::equals(user_id.clone()),
			read_progress::page::gt(0),
		])])
		.with(media::read_progresses::fetch(vec![
			read_progress::user_id::equals(user_id),
			read_progress::page::gt(0),
		]))
		.order_by(media::name::order(Direction::Asc))
		.exec()
		.await?
		.into_iter()
		.filter(|m| match m.read_progresses() {
			// Read progresses relation on media is one to many, there is a dual key
			// on read_progresses table linking a user and media. Therefore, there should
			// only be 1 item in this vec for each media resulting from the query.
			Ok(progresses) => {
				if progresses.len() != 1 {
					return false;
				}

				let progress = &progresses[0];

				if let Some(_epubcfi) = progress.epubcfi.as_ref() {
					// TODO: figure something out... might just need a `completed` field in progress TBH.
					return false;
				} else {
					return progress.page < m.pages;
				}
			},
			_ => false,
		});

	let entries: Vec<OpdsEntry> = media.into_iter().map(|m| OpdsEntry::from(m)).collect();

	let feed = OpdsFeed::new(
		"keepReading".to_string(),
		"Keep Reading".to_string(),
		Some(vec![
			OpdsLink {
				link_type: OpdsLinkType::Navigation,
				rel: OpdsLinkRel::ItSelf,
				href: String::from("/opds/v1.2/keep-reading"),
			},
			OpdsLink {
				link_type: OpdsLinkType::Navigation,
				rel: OpdsLinkRel::Start,
				href: String::from("/opds/v1.2/catalog"),
			},
		]),
		entries,
	);

	Ok(Xml(feed.build()?))
}

async fn libraries(Extension(ctx): State) -> ApiResult<Xml> {
	let db = ctx.get_db();

	let libraries = db.library().find_many(vec![]).exec().await?;

	let entries = libraries.into_iter().map(|l| OpdsEntry::from(l)).collect();

	let feed = OpdsFeed::new(
		"allLibraries".to_string(),
		"All libraries".to_string(),
		Some(vec![
			OpdsLink {
				link_type: OpdsLinkType::Navigation,
				rel: OpdsLinkRel::ItSelf,
				href: String::from("/opds/v1.2/libraries"),
			},
			OpdsLink {
				link_type: OpdsLinkType::Navigation,
				rel: OpdsLinkRel::Start,
				href: String::from("/opds/v1.2/catalog"),
			},
		]),
		entries,
	);

	Ok(Xml(feed.build()?))
}

// #[get("/libraries/<id>?<page>")]
// async fn library_by_id(
// 	Extension(ctx): State,
// 	id: String,
// 	page: Option<i64>,
// 	session: ReadableSession,
// ) -> ApiResult<Xml> {
// 	let db = ctx.get_db();

// 	let page = page.unwrap_or(0);
// 	let (skip, take) = pagination_bounds(page, 20);

// 	// FIXME: PCR doesn't support relation counts yet!
// 	let series_count = db
// 		.series()
// 		.count(vec![series::library_id::equals(Some(id.clone()))])
// 		.exec()
// 		.await?;

// 	let library = db
// 		.library()
// 		.find_unique(library::id::equals(id.clone()))
// 		.with(library::series::fetch(vec![]).skip(skip).take(take))
// 		.exec()
// 		.await?;

// 	if library.is_none() {
// 		return Err(ApiError::NotFound(format!("Library {} not found", id)));
// 	}

// 	let library = library.unwrap();

// 	Ok(Xml(
// 		OpdsFeed::paginated(
// 			library.id.as_str(),
// 			library.name.as_str(),
// 			format!("libraries/{}", &library.id).as_str(),
// 			library.series().unwrap_or(&Vec::new()).to_owned(),
// 			page,
// 			series_count,
// 		)
// 		.build()?,
// 	))
// }

// /// A handler for GET /opds/v1.2/series, accepts a `page` URL param. Note: OPDS
// /// pagination is zero-indexed.
// #[get("/series?<page>")]
// async fn get_series(page: Option<i64>, Extension(ctx): State, session: ReadableSession) -> ApiResult<Xml> {
// 	let db = ctx.get_db();

// 	let page = page.unwrap_or(0);
// 	let (skip, take) = pagination_bounds(page, 20);

// 	// FIXME: like other areas throughout Stump's paginated routes, I do not love
// 	// that I need to make 2 queries. Hopefully this get's better as prisma client matures
// 	// and introduces potential other work arounds.

// 	let series_count = db.series().count(vec![]).exec().await?;

// 	let series = db
// 		.series()
// 		.find_many(vec![])
// 		.skip(skip)
// 		.take(take)
// 		.exec()
// 		.await?;

// 	Ok(Xml(
// 		OpdsFeed::paginated(
// 			"allSeries",
// 			"All Series",
// 			"series",
// 			series,
// 			page,
// 			series_count,
// 		)
// 		.build()?,
// 	))
// }

// #[get("/series/latest?<page>")]
// async fn series_latest(
// 	page: Option<i64>,
// 	Extension(ctx): State,
// 	session: ReadableSession,
// ) -> ApiResult<Xml> {
// 	let db = ctx.get_db();

// 	let page = page.unwrap_or(0);
// 	let (skip, take) = pagination_bounds(page, 20);

// 	let series_count = db.series().count(vec![]).exec().await?;

// 	let series = db
// 		.series()
// 		.find_many(vec![])
// 		.order_by(prisma::series::updated_at::order(Direction::Desc))
// 		.skip(skip)
// 		.take(take)
// 		.exec()
// 		.await?;

// 	Ok(Xml(
// 		OpdsFeed::paginated(
// 			"latestSeries",
// 			"Latest Series",
// 			"series/latest",
// 			series,
// 			page,
// 			series_count,
// 		)
// 		.build()?,
// 	))
// }

// #[get("/series/<id>?<page>")]
// async fn series_by_id(
// 	id: String,
// 	page: Option<i64>,
// 	Extension(ctx): State,
// 	session: ReadableSession,
// ) -> ApiResult<Xml> {
// 	let db = ctx.get_db();

// 	let page = page.unwrap_or(0);
// 	let (skip, take) = pagination_bounds(page, 20);

// 	// FIXME: PCR doesn't support relation counts yet!
// 	// let series_media_count = db
// 	// 	.media()
// 	// 	.count(vec![media::series_id::equals(Some(id.clone()))])
// 	// 	.exec()
// 	// 	.await?;

// 	let series_media_count = db.media_in_series_count(id.clone()).await?;

// 	let series = db
// 		.series()
// 		.find_unique(series::id::equals(id.clone()))
// 		.with(
// 			series::media::fetch(vec![])
// 				.skip(skip)
// 				.take(take)
// 				.order_by(media::name::order(Direction::Asc)),
// 		)
// 		.exec()
// 		.await?;

// 	if series.is_none() {
// 		return Err(ApiError::NotFound(format!("Series {} not found", id)));
// 	}

// 	let series = series.unwrap();

// 	Ok(Xml(
// 		OpdsFeed::paginated(
// 			series.id.as_str(),
// 			series.name.as_str(),
// 			format!("series/{}", &series.id).as_str(),
// 			series.media().unwrap_or(&Vec::new()).to_owned(),
// 			page,
// 			series_media_count,
// 		)
// 		.build()?,
// 	))
// }

// #[get("/books/<id>/thumbnail")]
// async fn book_thumbnail(id: String, Extension(ctx): State, session: ReadableSession) -> ApiResult<ImageResponse> {
// 	let db = ctx.get_db();

// 	let book = db
// 		.media()
// 		.find_unique(media::id::equals(id.clone()))
// 		.exec()
// 		.await?;

// 	if book.is_none() {
// 		return Err(ApiError::NotFound(format!("Book {} not found", &id)));
// 	}

// 	let book = book.unwrap();

// 	Ok(media_file::get_page(&book.path, 1)?)
// }

// // TODO: generalize the function call
// // TODO: cache this? Look into this, I can send a cache-control header to the client, but not sure if I should
// // also cache on server. Check my types::rocket crate
// #[get("/books/<id>/pages/<page>?<zero_based>")]
// async fn book_page(
// 	id: String,
// 	page: usize,
// 	zero_based: Option<bool>,
// 	Extension(ctx): State,
// 	session: ReadableSession,
// ) -> ApiResult<ImageResponse> {
// 	let db = ctx.get_db();

// 	let zero_based = zero_based.unwrap_or(false);

// 	let book = db
// 		.media()
// 		.find_unique(media::id::equals(id.clone()))
// 		.exec()
// 		.await?;

// 	let mut correct_page = page;

// 	if zero_based {
// 		correct_page = page + 1;
// 	}

// 	if book.is_none() {
// 		return Err(ApiError::NotFound(format!("Book {} not found", &id)));
// 	}

// 	let book = book.unwrap();

// 	if book.path.ends_with(".epub") && correct_page == 1 {
// 		return Ok(epub::get_epub_cover(&book.path)?);
// 	}

// 	Ok(media_file::get_page(&book.path, correct_page as i32)?)
// }
