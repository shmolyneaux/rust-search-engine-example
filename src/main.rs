#![feature(plugin)]
#![plugin(rocket_codegen)]

extern crate rocket;

#[macro_use]
extern crate rocket_contrib;

#[macro_use]
extern crate serde_derive;

use rocket::data::FromData;
use rocket::response::NamedFile;
use rocket_contrib::{Json, Value};
use std::str::FromStr;

#[get("/api/search/<query>")]
fn search(query: String) -> Json<Value> {
    Json(Value::from_str(include_str!("../res/example.json")).unwrap())
}

#[get("/api/search")]
fn search_examples() -> Json<Value> {
    Json(Value::from_str(include_str!("../res/example.json")).unwrap())
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
    rocket::ignite().mount("/", routes![search, search_examples, favicon, index]).launch();
}
