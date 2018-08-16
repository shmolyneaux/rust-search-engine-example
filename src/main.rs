#![feature(plugin)]
#![plugin(rocket_codegen)]

extern crate rocket;

#[macro_use]
extern crate rocket_contrib;

#[macro_use]
extern crate tantivy;

#[macro_use]
extern crate lazy_static;

#[macro_use]
extern crate serde_derive;
extern crate serde;
extern crate serde_json;

use rocket::response::status::BadRequest;
use rocket::response::NamedFile;
use rocket_contrib::{Json, Value};
use tantivy::{collector::TopCollector, query::QueryParser, schema::*, Index};

const INDEX_PATH: &str = "index";

lazy_static! {
    static ref SCHEMA: Schema = {
        let mut schema_builder = SchemaBuilder::default();
        schema_builder.add_text_field("title", TEXT | STORED);
        schema_builder.add_text_field("body", TEXT | STORED);

        schema_builder.build()
    };
    static ref TITLE_FIELD: tantivy::schema::Field = { SCHEMA.get_field("title").unwrap() };
    static ref BODY_FIELD: tantivy::schema::Field = { SCHEMA.get_field("body").unwrap() };
    static ref INDEX: Index = {
        Index::open_in_dir(INDEX_PATH).expect(&format!("Could not open index in {}", INDEX_PATH))
    };
}

fn query(query: &str) -> tantivy::Result<Vec<rocket_contrib::Value>> {
    // The lazy_static macro creates references. We dereference them here so
    // that we can use the fields directly
    let title = *TITLE_FIELD;
    let body = *BODY_FIELD;

    INDEX.load_searchers()?;
    let searcher = INDEX.searcher();

    let query_parser = QueryParser::for_index(&INDEX, vec![title, body]);
    let query = query_parser.parse_query(&query)?;

    let mut top_collector = TopCollector::with_limit(7);
    searcher.search(&*query, &mut top_collector)?;

    let doc_addresses = top_collector.docs();

    println!("Found {} hits", doc_addresses.len());
    let mut vec: Vec<Value> = Vec::new();
    for doc_address in doc_addresses {
        let retrieved_doc = searcher.doc(&doc_address)?;
        let default = tantivy::schema::Value::Str("".to_string());

        let title_val = retrieved_doc.get_first(title).unwrap_or(&default);
        let body_val = retrieved_doc.get_first(body).unwrap_or(&default);

        let title_text = title_val.text();
        let body_text = body_val.text();

        vec.push(json!({
                "title": title_text,
                "snippet": body_text,
                "spans": [],
            }));
    }

    Ok(vec)
}

#[derive(Clone, Serialize, Deserialize, Debug)]
struct Post {
    title: String,
    body: String
}

fn wrapped_bad_request(s: &str) -> BadRequest<Json<Value>> {
    BadRequest(Some(Json(
        json!({ "status_code": 400, "message": s }),
    )))
}

#[post(
    "/api/document",
    format = "application/json",
    data = "<input_doc>"
)]
fn post_document(input_doc: Json<Post>) -> Result<Json<Value>, BadRequest<Json<Value>>> {
    let title: &str = &input_doc.title;
    let body: &str = &input_doc.body;
    match write_document(doc!(*TITLE_FIELD => title, *BODY_FIELD => body)) {
        Err(e) => {
            return Err(wrapped_bad_request(&format!(
                "Internal server error which I'm blaming on myself, sorry: {}",
                e
            )))
        }
        _ => (),
    };

    Ok(Json(json!({})))
}

fn write_document(doc: tantivy::Document) -> tantivy::Result<()>{
    const THREAD_BUFFER_SIZE_BYTES: usize = 50_000_000;

    let mut index_writer = INDEX.writer(THREAD_BUFFER_SIZE_BYTES)?;
    index_writer.add_document(doc);
    index_writer.commit()?;
    index_writer.wait_merging_threads()?;

    Ok(())
}

#[get("/api/search/<query_string>")]
fn search(query_string: String) -> Result<Json<Value>, BadRequest<Json<Value>>> {
    query(&query_string)
        .map(|results| Json(json!(results)))
        .map_err(|_| wrapped_bad_request("Invalid query string"))
}

#[get("/api/search")]
fn empty_search() -> Json<Value> {
    Json(json!(query("Example search").unwrap()))
}

#[get("/favicon.ico")]
fn favicon() -> NamedFile {
    NamedFile::open("res/favicon.ico").unwrap()
}

#[get("/")]
fn index() -> NamedFile {
    NamedFile::open("res/index.html").unwrap()
}

fn main() {
    rocket::ignite()
        .mount("/", routes![search, empty_search, post_document, favicon, index])
        .launch();
}
