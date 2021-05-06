#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate log;
#[macro_use]
extern crate serde_json;

use std::str::FromStr;
use std::sync::Arc;

use futures::stream::StreamExt;
use handlebars::Handlebars;
use mongodb::options::{FindOneOptions, FindOptions};
use rand::{Rng, thread_rng};
use rand::distributions::Alphanumeric;
use regex::Regex;
use tide::{Request, Response, ResponseBuilder};
use tide::utils::After;

use crate::types::{ChunkDoc, HeaderDoc};

mod types;

const PAGE_HTML: &str = include_str!("assets/page.html");
const FAVICON: &[u8] = include_bytes!("assets/favicon.ico");
const GITHUB_LOGO: &[u8] = include_bytes!("assets/github.png");

struct State {
    handlebars: Handlebars<'static>,
    database: Database,
    upload_password: String,
    chunks_size: u32,
}

struct Database {
    header_coll: mongodb::Collection<types::HeaderDoc>,
    chunk_coll: mongodb::Collection<types::ChunkDoc>,
}

lazy_static! {
    static ref ID_REGEX: Regex = Regex::new(r"^[a-zA-Z0-9]{2,}$").unwrap();
}

#[async_std::main]
async fn main() -> anyhow::Result<()> {
    use std::env::var;

    if let Err(..) = var("RUST_LOG") {
        std::env::set_var("RUST_LOG", "info");
    }
    pretty_env_logger::init();

    let mongo_uri = var("MONGO_URI").expect("expected MONGO_URI env var");
    let mongo_db = var("MONGO_DB").expect("expected MONGO_DB env var");
    let mongo_header_coll = var("MONGO_HEADER_COLL").expect("expected MONGO_HEADER_COLL env var");
    let mongo_chunk_coll = var("MONGO_CHUNK_COLL").expect("expected MONGO_CHUNK_COLL env var");
    let bind = var("BIND").expect("expected BIND env var");
    let upload_password = var("UPLOAD_PW").expect("expected UPLOAD_PW env var");
    let chunks_size = var("CHUNKS_SIZE").unwrap_or(String::from("16000000")); // ~16MB
    let chunks_size = u32::from_str(&chunks_size)?;

    if chunks_size > 16000000 {
        panic!("MongoDB only supports document sizes up to 16MB!");
    }

    let mut handlebars = Handlebars::new();
    handlebars.register_template_string("page", PAGE_HTML)?;

    let mongo = mongodb::Client::with_uri_str(&mongo_uri).await?;
    let mongodb = mongo.database(&mongo_db);
    let header_coll = mongodb.collection::<types::HeaderDoc>(&mongo_header_coll);
    let chunk_coll = mongodb.collection::<types::ChunkDoc>(&mongo_chunk_coll);

    let database = Database {
        header_coll,
        chunk_coll,
    };

    let state = Arc::new(State { handlebars, database, upload_password, chunks_size });

    let mut app = tide::with_state(state);

    app.at("/:id/d/:dkey")
        .get(delete_image);

    app.at("/:id")
        .options(get_image)
        .get(get_image);

    app.at("/u")
        .post(upload_image);

    app.with(After(|mut res: Response| async {
        if let Some(err) = res.error() {
            eprintln!("Error: {}", &err);
            res.set_body(json!({"error": "internal_server_error"}));
            res.set_status(500);
        }

        Ok(res)
    }));


    app.listen(bind).await?;

    Ok(())
}


async fn get_image(req: Request<Arc<State>>) -> tide::Result {
    let id = req.param("id")?;
    let state = req.state();


    // Images
    if id == "favicon.ico" {
        return Ok(Response::builder(200).body(FAVICON).content_type(tide::http::mime::ICO).build());
    }

    if id == "github.png" {
        return Ok(Response::builder(200).body(GITHUB_LOGO).content_type(tide::http::mime::PNG).build());
    }

    let split: Vec<&str> = id.split(".").collect();
    let id = split[0];

    let mut response = tide::Response::builder(200);

    let header = get_header(id, &state).await?;
    if let None = header {
        return Ok(response.body(json!({"error": "not_found"})).build());
    }
    let header = header.unwrap();

    response = write_header(&header, response);

    if req.method() == tide::http::Method::Options {
        return Ok(response.build());
    }

    if split.len() > 1 || !header.content_type.starts_with("image/") {
        let chunks = get_chunks(id, &state).await?;
        if let None = chunks {
            return Ok(response.body(json!({"error": "chunk_error"})).build());
        }

        let chunks: Vec<Vec<u8>> = chunks.unwrap().into_iter().map(|x| x.data).collect();
        let chunks = chunks.concat();

        info!("Serving file {}", &id);
        return Ok(response.body(chunks).build());
    }

    let mut extension = header.file_extension.as_str();
    if extension.is_empty() {
        extension = "ukn";
    }

    response = response.header(tide::http::headers::CONTENT_TYPE, "text/html");

    Ok(response.body(state.handlebars.render("page", &json!({"filename": format!("{}.{}", &id, extension)}))?).build())
}

async fn delete_image(req: Request<Arc<State>>) -> tide::Result {
    let id = req.param("id")?;
    let dkey = req.param("dkey")?;
    let state = req.state();

    let header = get_header(id, state).await?;
    if let None = header {
        return Ok(Response::builder(404).body(json!({"error": "not_found"})).build());
    }
    let header = header.unwrap();

    if header.delete_key != dkey {
        return Ok(Response::builder(400).body(json!({"error": "invalid_dkey"})).build());
    }

    info!("Deleting image {}", &header.id);
    delete_db_image(header, state).await?;
    Ok(Response::builder(200).body(json!({"error": serde_json::value::Value::Null})).build())
}

#[derive(serde::Deserialize, Default)]
struct UploadQuery {
    id: String,
}

async fn upload_image(mut req: Request<Arc<State>>) -> tide::Result {
    let state = req.state().clone();
    let auth_header = req.header(tide::http::headers::AUTHORIZATION);
    let custom_id = req.query::<UploadQuery>()?.id;

    if let None = auth_header {
        return Ok(tide::Response::builder(tide::http::StatusCode::Unauthorized).body(json!({"error": "unauthorized"})).build());
    }
    let auth_header = auth_header.unwrap();

    if state.upload_password != auth_header.get(0).unwrap().as_str() {
        return Ok(tide::Response::builder(tide::http::StatusCode::Unauthorized).body(json!({"error": "unauthorized"})).build());
    }

    let chunk_size = state.chunks_size as usize;
    let body = req.take_body().into_bytes().await?;

    if body.is_empty() {
        return Ok(tide::Response::builder(400).body(json!({"error": "empty_body"})).build());
    }

    let content_length = body.len();
    let typ = infer::get(&body);

    if let None = typ {
        return Ok(tide::Response::builder(400).body(json!({"error": "invalid_file_type"})).build());
    }
    let typ = typ.unwrap();


    let needed_chunks = (body.len() as f32 / chunk_size as f32).ceil() as usize;
    let mut chunks = Vec::with_capacity(needed_chunks);

    let mut read = 0;
    for i in 0..needed_chunks {
        let v = body[read..(chunk_size * (i + 1)).min(body.len())].to_vec();
        read += v.len();

        chunks.push(v);
    }

    let mut id: String;
    if custom_id.is_empty() {
        loop {
            id = thread_rng().sample_iter(&Alphanumeric).take(6).map(char::from).collect();

            if let None = get_header(&id, &state).await? {
                break;
            }
        }
    } else {
        if let Some(_) = get_header(&custom_id, &state).await? {
            return Ok(tide::Response::builder(400).body(json!({"error": "id_taken"})).build());
        }
        id = custom_id;
    }

    if !ID_REGEX.is_match(&id) || id == "github" || id == "favicon" {
        return Ok(Response::builder(400).body(json!({"error": "invalid_id"})).build());
    }

    let dkey: String = thread_rng().sample_iter(&Alphanumeric).take(32).map(char::from).collect();

    let header = types::HeaderDoc {
        id,
        delete_key: dkey,
        total_chunks: needed_chunks as u32,
        content_type: typ.mime_type().to_string(),
        file_extension: typ.extension().to_string(),
        content_length: content_length as u32,
        uploaded_at: std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis() as u64,
    };

    upload_db_image(header.clone(), chunks, &state).await?;

    info!("Image uploaded with id={} total_size={} total_chunks={} file_type={}", &header.id, &header.content_length, &header.total_chunks, &header.content_type);

    Ok(tide::Response::builder(200).body(json!({
        "id": &header.id,
        "deleteKey": &header.delete_key,
        "totalChunks": needed_chunks,
        "contentType": typ.mime_type(),
        "fileExtension": typ.extension()
    })).build())
}

fn write_header(header: &types::HeaderDoc, response: ResponseBuilder) -> ResponseBuilder {
    response.header(tide::http::headers::CONTENT_TYPE, &header.content_type)
        .header(tide::http::headers::CONTENT_LENGTH, &header.content_length.to_string())
        .header("X-ID", &header.id)
        .header("X-CHUNKS", header.total_chunks.to_string())
        .header("X-UPLOADED-AT", header.uploaded_at.to_string())
}

async fn get_header(id: &str, state: &Arc<State>) -> anyhow::Result<Option<types::HeaderDoc>> {
    state.database.header_coll.find_one(bson::doc! {"_id": &id}, FindOneOptions::default()).await.map_err(|e| anyhow::Error::from(e))
}

async fn get_chunks(id: &str, state: &Arc<State>) -> anyhow::Result<Option<Vec<types::ChunkDoc>>> {
    let a = state.database.chunk_coll.find(bson::doc! {"parent_id": &id}, FindOptions::default()).await?;
    let v: Vec<mongodb::error::Result<ChunkDoc>> = a.collect().await;

    if v.len() == 0 {
        return Ok(None);
    }

    return Ok(Some(v.into_iter().map(|x| x.unwrap()).collect()));
}

async fn delete_db_image(header: HeaderDoc, state: &Arc<State>) -> anyhow::Result<()> {
    state.database.header_coll.delete_one(bson::doc! {"_id": &header.id}, None).await?;
    state.database.chunk_coll.delete_many(bson::doc! {"parent_id": &header.id}, None).await?;

    Ok(())
}

async fn upload_db_image(header: HeaderDoc, body: Vec<Vec<u8>>, state: &Arc<State>) -> anyhow::Result<()> {
    let id = header.id.to_string();
    state.database.header_coll.insert_one(header, None).await?;
    state.database.chunk_coll.insert_many(body.into_iter().enumerate().map(|(i, c)| ChunkDoc { index: i as i32, parent_id: id.clone(), data: c }), None).await?;
    Ok(())
}