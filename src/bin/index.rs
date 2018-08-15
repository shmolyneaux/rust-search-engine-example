extern crate bzip2;
extern crate serde_json;
extern crate tantivy;

use std::io::{BufRead, BufReader};
use std::path::Path;
use tantivy::{schema::*, Index};

use serde_json::Value;

fn run_indexer(src_path: &Path, dest_path: &Path) -> tantivy::Result<()> {
    let mut schema_builder = SchemaBuilder::default();
    schema_builder.add_text_field("title", TEXT | STORED);
    schema_builder.add_text_field("body", TEXT | STORED);

    let schema = schema_builder.build();
    let index = Index::create_in_dir(dest_path, schema.clone())?;

    const THREAD_BUFFER_SIZE_BYTES: usize = 50_000_000;
    let mut index_writer = index.writer(THREAD_BUFFER_SIZE_BYTES)?;

    let title = schema.get_field("title").unwrap();
    let body = schema.get_field("body").unwrap();

    let docs = get_documents(src_path, &title, &body);
    for doc in docs {
        index_writer.add_document(doc);
    }

    index_writer.commit()?;
    index_writer.wait_merging_threads()?;

    Ok(())
}

fn get_documents(parent_directory: &Path, title: &Field, body: &Field) -> Vec<Document> {
    let mut vec = Vec::new();
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
            let mut child_documents = get_documents(&path, title, body);
            vec.append(&mut child_documents);
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
            let author: &str = val.get("author").unwrap().as_str().unwrap();
            let contents: &str = val.get("body").unwrap().as_str().unwrap();
            let mut doc = Document::default();
            doc.add_text(*title, author);
            doc.add_text(*body, &contents);
            vec.push(doc);
        }
    }
    vec
}

fn main() {
    println!("Starting indexing");
    run_indexer(Path::new("corpus"), Path::new("index")).unwrap();
}
