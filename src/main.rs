#![feature(plugin)]
#![plugin(rocket_codegen)]

extern crate rocket;

#[macro_use]
extern crate rocket_contrib;
extern crate tantivy;

use rocket::response::NamedFile;
use rocket_contrib::{Json, Value};
use std::path::Path;
use tantivy::{collector::TopCollector, query::QueryParser, schema::*, Index};

fn query(index_path: &Path, query: &str) -> tantivy::Result<Vec<rocket_contrib::Value>> {
    let mut schema_builder = SchemaBuilder::default();
    schema_builder.add_text_field("title", TEXT | STORED);
    schema_builder.add_text_field("body", TEXT | STORED);

    let schema = schema_builder.build();
    let title = schema.get_field("title").unwrap();
    let body = schema.get_field("body").unwrap();

    let index = Index::open_in_dir(index_path)?;
    index.load_searchers()?;
    let searcher = index.searcher();

    let query_parser = QueryParser::for_index(&index, vec![title, body]);
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
    Json(json!(query(Path::new("index"), &query_string).unwrap()))
}

#[get("/api/search")]
fn empty_search() -> Json<Value> {
    Json(json!(query(Path::new("index"), "Example search").unwrap()))
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
