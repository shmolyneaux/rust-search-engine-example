extern crate bzip2;
#[macro_use]
extern crate serde_json;

#[macro_use]
extern crate tantivy;

extern crate ureq;

use std::io::{BufRead, BufReader};
use std::path::Path;
use tantivy::{schema::*, Index};

use serde_json::Value;

#[derive(Debug)]
struct Post {
    title: String,
    body: String,
}

fn process_documents(parent_directory: &Path, cb: fn(Post) -> ()) {
    for entry in std::fs::read_dir(parent_directory).unwrap() {
        let entry = match entry {
            Ok(entry) => entry,
            Err(e) => {
                eprintln!("Error while getting directory entry: {}", e);
                continue;
            }
        };

        let path = entry.path();
        if path.is_dir() {
            process_documents(&path, cb);
            continue;
        }

        println!("{:?}", path);

        let mut file = match std::fs::File::open(entry.path()) {
            Ok(f) => f,
            Err(e) => {
                eprintln!("Error while opening {:?}: {}", entry.path(), e);
                continue;
            }
        };

        let mut zip = BufReader::new(bzip2::bufread::BzDecoder::new(BufReader::new(file)));

        for line in zip.lines() {
            let val: Value = serde_json::from_str(&line.unwrap()).unwrap();
            let title: &str = val.get("author").unwrap().as_str().unwrap();
            let body: &str = val.get("body").unwrap().as_str().unwrap();
            cb(Post {title: title.to_string(), body: body.to_string()});
        }
    }
}

fn main() {
    process_documents(Path::new("corpus"), |post| {
        let resp = ureq::post("http://localhost:8000/api/document")
            .set("Content-Type", "application/json")
            .send_json(json!({
                "title": post.title,
                "body": post.body
            }));

        if resp.ok() {
            println!("Sent {:?}", post);
        } else {
            eprintln!("Error while posting {:?}: {:?}", post, resp);
        }
    });
}
