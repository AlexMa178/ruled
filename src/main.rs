mod types;
mod type_names;
mod state;
mod lexer;
mod parser;

use std::path::PathBuf;
use std::thread::Builder;
use std::{ env, fs };

use crate::parser::Expr;
use crate::state::State;

fn main() {
    let mut exprs = Vec::new();
    let paths = env::args().skip(1).flat_map(|arg| {
        if let Ok(dir) = fs::read_dir(&arg) {
            let mut vec = dir.filter_map(|maybe_entry|
                maybe_entry.ok().map(|entry| entry.path())
            ).collect::<Vec<_>>();
            vec.sort();
            vec
        } else {
            vec![ PathBuf::from(arg) ]
        }
    }).collect::<Vec<_>>();
    for path in paths {
        let program_str = match fs::read_to_string(path) {
            Ok(s) => s,
            Err(err) => { eprintln!("file error: {err}"); return; },
        };
        let program_exprs = match exprs_of(&program_str) {
            Ok(e) => e,
            Err(err) => { eprintln!("syntax error: {err}"); return; },
        };
        exprs.extend(program_exprs);
    }
    if let Err(err) = run(exprs) {
        eprintln!("runtime error: {err}");
    }
}

fn exprs_of(program_str: &str) -> Result<Vec<Expr>, String> {
    let mut tokens = lexer::lexer(program_str)?;
    let mut exprs = Vec::new();
    while !tokens.is_empty() {
        exprs.push(Expr::from_tokens(&mut tokens)?);
    }
    Ok(exprs)
}

fn run(exprs: Vec<Expr>) -> Result<(), String> {
    Builder::new().name("program thread".to_owned()).stack_size(8 * 2usize.pow(20)).spawn(|| {
        let mut state = State::new();
        for expr in exprs {
            expr.run(&mut state)?;
        }
        Ok(())
    }).expect("thread error").join().expect("join error")
}