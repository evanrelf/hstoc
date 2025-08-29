use clap::Parser as _;
use rayon::prelude::*;
use std::{
    fs,
    io::{self, Write as _},
    path::PathBuf,
};
use tree_sitter::{Language, Node, Parser, QueryCursor, StreamingIterator as _, Tree};

#[derive(clap::Parser)]
struct Args {
    #[arg(long)]
    query: Query,
    #[arg(long)]
    stdin_path: Option<PathBuf>,
    paths: Vec<PathBuf>,
}

#[derive(clap::ValueEnum, Clone)]
enum Query {
    Imports,
    Exports,
    ExplicitExports,
    Declarations,
    DataType,
    Newtype,
    TypeSynonym,
    Class,
    TypeFamily,
    Function,
    FunctionInfix,
    Bind,
}

struct Context {
    language: Language,
    source_code: String,
    tree: Tree,
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let query = match args.query {
        Query::Imports => query_imports,
        Query::Exports => query_exports,
        Query::ExplicitExports => query_explicit_exports,
        Query::Declarations => query_declarations,
        Query::DataType => query_data_type,
        Query::Newtype => query_newtype,
        Query::TypeSynonym => query_type_synonym,
        Query::Class => query_class,
        Query::TypeFamily => query_type_family,
        Query::Function => query_function,
        Query::FunctionInfix => query_function_infix,
        Query::Bind => query_bind,
    };
    let language = tree_sitter_haskell::LANGUAGE.into();
    let process = |path: &str, source_code: &str| {
        let source_code = source_code.to_string();
        let mut parser = Parser::new();
        parser.set_language(&language)?;
        let tree = parser.parse(&source_code, None).unwrap();
        let cx = Context {
            language: language.clone(),
            source_code,
            tree,
        };
        let mut stdout = io::stdout();
        for node in query(&cx)? {
            let range = node.range();
            let line = range.start_point.row;
            let column = range.start_point.column;
            let text = node_text(&cx, &node).unwrap();
            writeln!(&mut stdout, "{path}:{line}:{column}:{text}")?;
        }
        stdout.flush()?;
        anyhow::Ok(())
    };
    if args.paths.is_empty() {
        let path = match args.stdin_path {
            Some(path) => path.display().to_string(),
            None => String::from("<stdin>"),
        };
        process(&path, &io::read_to_string(io::stdin())?)?;
    } else {
        args.paths.par_iter().try_for_each(|path| {
            process(&path.display().to_string(), &fs::read_to_string(path)?)
        })?;
    }
    Ok(())
}

fn query_imports(cx: &Context) -> anyhow::Result<Vec<Node<'_>>> {
    query(cx, "(haskell (imports (import module: (_) @import)))")
}

fn query_exports(cx: &Context) -> anyhow::Result<Vec<Node<'_>>> {
    let explicit = query_explicit_exports(cx)?;
    if explicit.is_empty() {
        query_declarations(cx)
    } else {
        Ok(explicit)
    }
}

fn query_explicit_exports(cx: &Context) -> anyhow::Result<Vec<Node<'_>>> {
    query(cx, "(haskell (header (exports export: (_) @export)))")
}

fn query_declarations(cx: &Context) -> anyhow::Result<Vec<Node<'_>>> {
    let mut nodes = query_data_type(cx)?;
    nodes.extend(query_newtype(cx)?);
    nodes.extend(query_type_synonym(cx)?);
    nodes.extend(query_class(cx)?);
    nodes.extend(query_type_family(cx)?);
    nodes.extend(query_function(cx)?);
    nodes.extend(query_function_infix(cx)?);
    nodes.extend(query_bind(cx)?);
    Ok(nodes)
}

fn query_data_type(cx: &Context) -> anyhow::Result<Vec<Node<'_>>> {
    query(
        cx,
        "(haskell (declarations (data_type name: (_) @data_type)))",
    )
}

fn query_newtype(cx: &Context) -> anyhow::Result<Vec<Node<'_>>> {
    query(cx, "(haskell (declarations (newtype name: (_) @newtype)))")
}

fn query_type_synonym(cx: &Context) -> anyhow::Result<Vec<Node<'_>>> {
    query(
        cx,
        "(haskell (declarations (type_synomym name: (_) @type_synonym)))",
    )
}

fn query_class(cx: &Context) -> anyhow::Result<Vec<Node<'_>>> {
    query(cx, "(haskell (declarations (class name: (_) @class)))")
}

fn query_type_family(cx: &Context) -> anyhow::Result<Vec<Node<'_>>> {
    query(
        cx,
        "(haskell (declarations (type_family name: (_) @type_family)))",
    )
}

fn query_function(cx: &Context) -> anyhow::Result<Vec<Node<'_>>> {
    query(
        cx,
        "(haskell (declarations (function name: (_) @function)))",
    )
}

fn query_function_infix(cx: &Context) -> anyhow::Result<Vec<Node<'_>>> {
    query(
        cx,
        "(haskell (declarations (function (infix operator: (_) @function))))",
    )
}

fn query_bind(cx: &Context) -> anyhow::Result<Vec<Node<'_>>> {
    query(cx, "(haskell (declarations (bind name: (_) @bind)))")
}

fn query<'a>(cx: &'a Context, query: &str) -> anyhow::Result<Vec<Node<'a>>> {
    let root_node = cx.tree.root_node();
    let query = tree_sitter::Query::new(&cx.language, query)?;
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
