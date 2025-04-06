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
    let source_code = if let Some(ref path) = args.path {
        fs::read_to_string(path)?
    } else {
        io::read_to_string(io::stdin())?
    };
    let tree = parser.parse(&source_code, None).unwrap();
    let root_node = tree.root_node();
    let query = Query::new(
        &language,
        r#"
        (haskell (declarations (data_type name: (_) @decl_name)))
        (haskell (declarations (newtype name: (_) @decl_name)))
        (haskell (declarations (type_synomym name: (_) @decl_name)))
        (haskell (declarations (class name: (_) @decl_name)))
        (haskell (declarations (type_family name: (_) @decl_name)))
        (haskell (declarations (function name: (_) @decl_name)))
        (haskell (declarations (function (infix operator: (_) @decl_name))))
        (haskell (declarations (bind name: (_) @decl_name)))
        "#,
    )?;
    let mut query_cursor = QueryCursor::new();
    let mut query_matches = query_cursor.matches(&query, root_node, source_code.as_bytes());
    while let Some(query_match) = query_matches.next() {
        for match_capture in query_match.captures {
            let range = match_capture.node.range();
            let path = match args.path {
                Some(ref path) => path.display().to_string(),
                None => String::from("<stdin>"),
            };
            let line = range.start_point.row;
            let column = range.start_point.column;
            let text = source_code.get(range.start_byte..range.end_byte).unwrap();
            println!("{path}:{line}:{column}:{text}");
        }
    }
    Ok(())
}
