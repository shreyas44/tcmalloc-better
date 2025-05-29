use anyhow::{Result, anyhow};
use std::cell::OnceCell;
use std::cmp::Reverse;
use std::collections::{BinaryHeap, HashSet};
use std::env;
use std::fs::File;
use std::io::{BufRead, BufReader};

#[derive(Default)]
struct CompileParams<'a> {
    defines: Vec<&'a str>,
    w_all: bool,
    w_extra: bool,
    w_error: bool,
    warns: Vec<&'a str>,
    flags: Vec<&'a str>,
    std: Option<&'a str>,
    i_quote: Vec<&'a str>,
    g: Option<&'a str>,
}

#[derive(Default)]
struct MergedCompileParams {
    defines: HashSet<String>,
    w_all: HashSet<bool>,
    w_extra: HashSet<bool>,
    w_error: HashSet<bool>,
    warns: HashSet<String>,
    flags: HashSet<String>,
    std: HashSet<Option<String>>,
    i_quote: HashSet<String>,
    g: HashSet<Option<String>>,
}

#[derive(Debug)]
#[allow(dead_code)]
struct SortedCompileParams {
    defines: Vec<String>,
    w_all: Vec<bool>,
    w_extra: Vec<bool>,
    w_error: Vec<bool>,
    warns: Vec<String>,
    flags: Vec<String>,
    std: Vec<Option<String>>,
    i_quote: Vec<String>,
    g: Vec<Option<String>>,
}

impl From<MergedCompileParams> for SortedCompileParams {
    fn from(value: MergedCompileParams) -> Self {
        SortedCompileParams {
            defines: sort(value.defines.into_iter().collect()),
            w_all: sort(value.w_all.into_iter().collect()),
            w_extra: sort(value.w_extra.into_iter().collect()),
            w_error: sort(value.w_error.into_iter().collect()),
            warns: sort(value.warns.into_iter().collect()),
            flags: sort(value.flags.into_iter().collect()),
            std: sort(value.std.into_iter().collect()),
            i_quote: sort(value.i_quote.into_iter().collect()),
            g: sort(value.g.into_iter().collect()),
        }
    }
}

fn sort<T: Ord>(mut v: Vec<T>) -> Vec<T> {
    v.sort();
    v
}

impl MergedCompileParams {
    fn merge(&mut self, compile_params: &CompileParams) {
        self.w_all.insert(compile_params.w_all);
        self.w_extra.insert(compile_params.w_extra);
        self.w_error.insert(compile_params.w_error);
        self.std.insert(compile_params.std.map(|s| s.to_string()));
        self.g.insert(compile_params.g.map(|s| s.to_string()));
        extend(&mut self.defines, &compile_params.defines);
        extend(&mut self.flags, &compile_params.flags);
        extend(&mut self.i_quote, &compile_params.i_quote);
        extend(&mut self.warns, &compile_params.warns);
    }
}

fn extend(set: &mut HashSet<String>, values: &[&str]) {
    for &value in values {
        if !set.contains(value) {
            set.insert(value.to_string());
        }
    }
}

// tool for analyzing bazel build logs
// bazel build -j 1 --subcommands //tcmalloc:tcmalloc 2> build.log
// cargo run -- [-q] build.log
fn main() -> Result<()> {
    let args: Vec<_> = env::args().skip(1).collect();
    let log_name = args.last().expect("expected build log name");
    let disable_per_line_output = args.iter().any(|arg| arg == "-q");
    let disable_file_list_output = args.iter().any(|arg| arg == "-l");
    let log_file = BufReader::new(File::open(log_name)?);
    let mut it = log_file.lines().fuse().peekable();
    let mut i = 0usize;
    let mut merged = MergedCompileParams::default();
    let mut source_files = BinaryHeap::new();
    while let Some(line) = it.next() {
        let line = line?;
        const SUBCOMMAND: &str = "SUBCOMMAND: # ";
        let line = match line.strip_prefix(SUBCOMMAND) {
            Some(line) => line,
            None => {
                continue;
            }
        };
        const ACTION_COMPILING: &str = " [action 'Compiling ";
        let line = match line.find(ACTION_COMPILING) {
            Some(action_compiling_pos) => &line[action_compiling_pos + ACTION_COMPILING.len()..],
            None => {
                continue;
            }
        };
        let source_file = match line.find('\'') {
            Some(quote_pos) => &line[..quote_pos],
            None => {
                continue;
            }
        };
        while let Some(line) = it.peek() {
            if line
                .as_ref()
                .ok()
                .filter(|line| line.starts_with(SUBCOMMAND))
                .is_some()
            {
                break;
            }
            // SAFETY: `it.peek()` ensures that `it.next()` is not `None`.
            let line = it.next().unwrap()?;
            if !line.contains(" -o ") {
                continue;
            }
            let mut it = line.split_ascii_whitespace();
            it.next().ok_or_else(|| anyhow!("expected compiler"))?;
            let mut args: Vec<_> = it.collect();
            {
                let last = args
                    .last_mut()
                    .ok_or_else(|| anyhow!("empty args for compiler"))?;
                *last = last
                    .strip_suffix(')')
                    .ok_or_else(|| anyhow!("not found ')' at the end of args"))?;
            }
            let input = OnceCell::new();
            let output = OnceCell::new();
            let mut compile_params = CompileParams::default();

            let mut it = args.iter();
            while let Some(&arg) = it.next() {
                let arg = strip_quotes(arg)?;
                match arg.as_bytes() {
                    [b'-', b'W', b'a', b'l', b'l'] => compile_params.w_all = true,
                    [b'-', b'W', b'e', b'x', b't', b'r', b'a'] => compile_params.w_extra = true,
                    [b'-', b'W', b'e', b'r', b'r', b'o', b'r'] => compile_params.w_error = true,
                    [b'-', b'o'] => output
                        .set(
                            it.next()
                                .copied()
                                .ok_or_else(|| anyhow!("expected value for -o"))?,
                        )
                        .map_err(|err| {
                            anyhow!("duplicate output: {err}, already was {output:?}")
                        })?,
                    [b'-', b'c']
                    | [b'-', b'M', b'D']
                    | [
                        b'-',
                        b'f',
                        b'r',
                        b'a',
                        b'n',
                        b'd',
                        b'o',
                        b'm',
                        b'-',
                        b's',
                        b'e',
                        b'e',
                        b'd',
                        b'=',
                        ..,
                    ] => {}
                    [b'-', b'M', b'F'] => {
                        it.next().ok_or_else(|| anyhow!("expected value for -MF"))?;
                    }
                    [b'-', b'i', b'q', b'u', b'o', b't', b'e'] => compile_params.i_quote.push(
                        it.next()
                            .copied()
                            .ok_or_else(|| anyhow!("expected value for -iqoute"))?,
                    ),
                    [b'-', b'D', tail @ ..] => {
                        compile_params.defines.push(unsafe {
                            //SAFETY: `tail` is a valid UTF-8 string.
                            str::from_utf8_unchecked(tail)
                        })
                    }
                    [b'-', b's', b't', b'd', b'=', tail @ ..] => {
                        compile_params.std.replace(strip_quotes(unsafe {
                            //SAFETY: `tail` is a valid UTF-8 string.
                            str::from_utf8_unchecked(tail)
                        })?);
                    }
                    [b'-', b'f', ..] | [b'-', b'U', ..] => compile_params.flags.push(arg),
                    [b'-', b'W', ..] => compile_params.warns.push(arg),
                    [b'-', b'g', tail @ ..] => {
                        compile_params.g.replace(strip_quotes(unsafe {
                            //SAFETY: `tail` is a valid UTF-8 string.
                            str::from_utf8_unchecked(tail)
                        })?);
                    }
                    _ => input
                        .set(arg)
                        .map_err(|err| anyhow!("duplicate input: {err}, already was {input:?}"))?,
                }
            }
            let input = input
                .into_inner()
                .ok_or_else(|| anyhow!("input not found"))?;
            let output = output
                .into_inner()
                .ok_or_else(|| anyhow!("output not found"))?;
            i += 1;
            merged.merge(&compile_params);
            if !disable_file_list_output {
                source_files.push(Reverse(source_file.to_string()));
            }
            if !disable_per_line_output {
                println!(
                    "{i} - {source_file}: {input} o:{output} std:{std:?} g:{g:?} wall:{w_all} \
                    wextra:{w_extra} werror:{w_error} D:{defines:?} F:{flags:?} W:{warns:?}\
                     I:{includes:?}",
                    std = compile_params.std,
                    w_all = compile_params.w_all,
                    w_extra = compile_params.w_extra,
                    w_error = compile_params.w_error,
                    defines = compile_params.defines,
                    flags = compile_params.flags,
                    includes = compile_params.i_quote,
                    warns = compile_params.warns,
                    g = compile_params.g,
                );
            }
        }
    }
    println!("{:#?}", SortedCompileParams::from(merged));
    if !disable_file_list_output {
        println!("[");
        while let Some(Reverse(source_file)) = source_files.pop() {
            println!("    \"{source_file}\",");
        }
        println!("]");
    }
    Ok(())
}

fn strip_quotes(s: &str) -> Result<&str> {
    Ok(match s.as_bytes() {
        [b'\'', tail @ ..] => unsafe {
            //SAFETY: `tail` is a valid UTF-8 string.
            str::from_utf8_unchecked(tail)
        }
        .strip_suffix('\'')
        .ok_or_else(|| anyhow!("arg without closing '"))?,
        [b'"', tail @ ..] => unsafe {
            //SAFETY: `tail` is a valid UTF-8 string.
            str::from_utf8_unchecked(tail)
        }
        .strip_suffix('"')
        .ok_or_else(|| anyhow!("arg without closing \""))?,
        _ => s,
    })
}
