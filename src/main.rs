use clap::Parser as _;
use std::{fs, path::PathBuf};
use tree_sitter::Parser;

#[derive(clap::Parser)]
struct Args {
    path: PathBuf,
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let mut parser = Parser::new();
    parser.set_language(&tree_sitter_haskell::LANGUAGE.into())?;
    let source_code = fs::read_to_string(&args.path)?;
    let tree = parser.parse(source_code, None).unwrap();
    let root_node = tree.root_node();
    println!("{}", root_node.to_sexp());
    Ok(())
}
