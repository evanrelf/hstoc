use clap::Parser as _;
use std::{fs, io, path::PathBuf};
use tree_sitter::{Parser, Query, QueryCursor, StreamingIterator as _};

#[derive(clap::Parser)]
struct Args {
    path: Option<PathBuf>,
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let language = tree_sitter_haskell::LANGUAGE.into();
    let mut parser = Parser::new();
    parser.set_language(&language)?;
    let source_code = if let Some(path) = args.path {
        fs::read_to_string(&path)?
    } else {
        io::read_to_string(io::stdin())?
    };
    let tree = parser.parse(&source_code, None).unwrap();
    let root_node = tree.root_node();
    let query = Query::new(
        &language,
        r#"
        (declarations (data_type name: (_) @decl_name))
        (declarations (newtype name: (_) @decl_name))
        (declarations (type_synomym name: (_) @decl_name))
        (declarations (class name: (_) @decl_name))
        (declarations (type_family name: (_) @decl_name))
        (declarations (function name: (_) @decl_name))
        (declarations (function (infix operator: (_) @decl_name)))
        (declarations (bind name: (_) @decl_name))
        "#,
    )?;
    let mut query_cursor = QueryCursor::new();
    let mut query_matches = query_cursor.matches(&query, root_node, source_code.as_bytes());
    while let Some(query_match) = query_matches.next() {
        for query_capture in query_match.captures {
            let range = query_capture.node.byte_range();
            let text = source_code.get(range).unwrap();
            println!("{text}");
        }
    }
    Ok(())
}
