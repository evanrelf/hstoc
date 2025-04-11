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
    query(cx, "(haskell (imports (import module: (_) @import)))")
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
    query(cx, "(haskell (header (exports export: (_) @export)))")
}

fn query_declarations(cx: &Context) -> anyhow::Result<Vec<Node>> {
    let mut nodes = query_data_type(cx)?;
    nodes.extend(query_newtype(cx)?);
    nodes.extend(query_type_synomym(cx)?);
    nodes.extend(query_class(cx)?);
    nodes.extend(query_type_family(cx)?);
    nodes.extend(query_function(cx)?);
    nodes.extend(query_function_infix(cx)?);
    nodes.extend(query_bind(cx)?);
    Ok(nodes)
}

fn query_data_type(cx: &Context) -> anyhow::Result<Vec<Node>> {
    query(
        cx,
        "(haskell (declarations (data_type name: (_) @data_type)))",
    )
}

fn query_newtype(cx: &Context) -> anyhow::Result<Vec<Node>> {
    query(cx, "(haskell (declarations (newtype name: (_) @newtype)))")
}

fn query_type_synomym(cx: &Context) -> anyhow::Result<Vec<Node>> {
    query(
        cx,
        "(haskell (declarations (type_synomym name: (_) @type_synomym)))",
    )
}

fn query_class(cx: &Context) -> anyhow::Result<Vec<Node>> {
    query(cx, "(haskell (declarations (class name: (_) @class)))")
}

fn query_type_family(cx: &Context) -> anyhow::Result<Vec<Node>> {
    query(
        cx,
        "(haskell (declarations (type_family name: (_) @type_family)))",
    )
}

fn query_function(cx: &Context) -> anyhow::Result<Vec<Node>> {
    query(
        cx,
        "(haskell (declarations (function name: (_) @function)))",
    )
}

fn query_function_infix(cx: &Context) -> anyhow::Result<Vec<Node>> {
    query(
        cx,
        "(haskell (declarations (function (infix operator: (_) @function))))",
    )
}

fn query_bind(cx: &Context) -> anyhow::Result<Vec<Node>> {
    query(cx, "(haskell (declarations (bind name: (_) @bind)))")
}

fn query<'a>(cx: &'a Context, query: &str) -> anyhow::Result<Vec<Node<'a>>> {
    let root_node = cx.tree.root_node();
    let query = Query::new(&cx.language, query)?;
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
