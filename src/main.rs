#![expect(dead_code)]

use clap::Parser as _;
use std::{fs, io, path::PathBuf};
use tree_sitter::{Language, Node, Parser, Query, QueryCursor, StreamingIterator as _, Tree};

#[derive(clap::Parser)]
struct Args {
    path: Option<PathBuf>,
}

struct Context {
    language: Language,
    source_code: String,
    tree: Tree,
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
    let cx = Context {
        language,
        source_code,
        tree,
    };
    for node in query_declarations(&cx)? {
        let range = node.range();
        let path = match args.path {
            Some(ref path) => path.display().to_string(),
            None => String::from("<stdin>"),
        };
        let line = range.start_point.row;
        let column = range.start_point.column;
        let text = node_text(&cx, &node).unwrap();
        println!("{path}:{line}:{column}:{text}");
    }
    Ok(())
}

fn query_imports(cx: &Context) -> anyhow::Result<Vec<Node>> {
    let root_node = cx.tree.root_node();
    let query = Query::new(
        &cx.language,
        r#"
        (haskell (imports (import module: (_) @import)))
        "#,
    )?;
    let mut query_cursor = QueryCursor::new();
    let mut query_matches = query_cursor.matches(&query, root_node, cx.source_code.as_bytes());
    let mut results = Vec::with_capacity(query_matches.size_hint().0);
    while let Some(query_match) = query_matches.next() {
        for match_capture in query_match.captures {
            results.push(match_capture.node);
        }
    }
    Ok(results)
}

fn query_exports(cx: &Context) -> anyhow::Result<Vec<Node>> {
    let explicit = query_explicit_exports(cx)?;
    if !explicit.is_empty() {
        Ok(explicit)
    } else {
        query_declarations(cx)
    }
}

fn query_explicit_exports(cx: &Context) -> anyhow::Result<Vec<Node>> {
    let root_node = cx.tree.root_node();
    let query = Query::new(
        &cx.language,
        r#"
        (haskell (header (exports export: (_) @export)))
        "#,
    )?;
    let mut query_cursor = QueryCursor::new();
    let mut query_matches = query_cursor.matches(&query, root_node, cx.source_code.as_bytes());
    let mut results = Vec::with_capacity(query_matches.size_hint().0);
    while let Some(query_match) = query_matches.next() {
        for match_capture in query_match.captures {
            results.push(match_capture.node);
        }
    }
    Ok(results)
}

fn query_declarations(cx: &Context) -> anyhow::Result<Vec<Node>> {
    let root_node = cx.tree.root_node();
    let query = Query::new(
        &cx.language,
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
    let mut query_matches = query_cursor.matches(&query, root_node, cx.source_code.as_bytes());
    let mut results = Vec::with_capacity(query_matches.size_hint().0);
    while let Some(query_match) = query_matches.next() {
        for match_capture in query_match.captures {
            results.push(match_capture.node);
        }
    }
    Ok(results)
}

fn node_text<'cx>(cx: &'cx Context, node: &Node) -> Option<&'cx str> {
    cx.source_code.get(node.byte_range())
}
