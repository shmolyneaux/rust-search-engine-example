#![feature(plugin)]
#![plugin(rocket_codegen)]

extern crate rocket;

#[macro_use]
extern crate rocket_contrib;
extern crate tantivy;

#[macro_use]
extern crate lazy_static;

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

#[get("/api/search/<query_string>")]
fn search(query_string: String) -> Json<Value> {
    Json(json!(query(&query_string).unwrap()))
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
        .mount("/", routes![search, empty_search, favicon, index])
        .launch();
}
