use anyhow::{Result, anyhow};
use clap::Parser;
use std::cell::OnceCell;
use std::collections::BTreeSet;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;

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

#[derive(Default, Debug)]
struct MergedCompileParams {
    defines: BTreeSet<String>,
    w_all: BTreeSet<bool>,
    w_extra: BTreeSet<bool>,
    w_error: BTreeSet<bool>,
    warns: BTreeSet<String>,
    flags: BTreeSet<String>,
    std: BTreeSet<Option<String>>,
    i_quote: BTreeSet<String>,
    g: BTreeSet<Option<String>>,
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

fn extend(set: &mut BTreeSet<String>, values: &[&str]) {
    for &value in values {
        if !set.contains(value) {
            set.insert(value.to_string());
        }
    }
}

/// Tool for analyzing bazel build logs.
///
/// Example to use:
///
/// Execute inside `path_to_tcmalloc_dir`
///
/// $ bazel clean
///
/// $ bazel build --subcommands 'target' 2> build.log
///
/// (where 'target'  is a bazel build target, for example: //tcmalloc:tcmalloc):
///
/// Then execute inside this crate:
///
/// $ cargo run -p bazel-log-parser -- path_to_tcmalloc_dir/build.log
#[derive(Parser)]
struct Args {
    /// Log filename to parse
    log_name: PathBuf,
    #[arg(short = 'q')]
    /// Disable per line output
    disable_per_line_output: bool,
    /// Disable file list output
    #[arg(short = 'l')]
    disable_file_list_output: bool,
}

fn main() -> Result<()> {
    let Args {
        log_name,
        disable_per_line_output,
        disable_file_list_output,
    } = Args::parse();
    let mut source_files = if disable_file_list_output {
        None
    } else {
        Some(BTreeSet::new())
    };
    let mut i = 0usize;
    let mut merged = MergedCompileParams::default();
    let mut it = BufReader::new(File::open(log_name)?).lines().peekable();
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
                    b"-Wall" => compile_params.w_all = true,
                    b"-Wextra" => compile_params.w_extra = true,
                    b"-Werror" => compile_params.w_error = true,
                    b"-o" => output
                        .set(
                            it.next()
                                .copied()
                                .ok_or_else(|| anyhow!("expected value for -o"))?,
                        )
                        .map_err(|err| {
                            anyhow!("duplicate output: {err}, already was {output:?}")
                        })?,
                    b"-c"
                    | b"-MD"
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
                    b"-MF" => {
                        it.next().ok_or_else(|| anyhow!("expected value for -MF"))?;
                    }
                    b"-iquote" => compile_params.i_quote.push(
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
            if !disable_per_line_output {
                println!(
                    "{i} - {source_file}: {input} o:{output} std:{std:?} g:{g:?} wall:{w_all} \
                    wextra:{w_extra} werror:{w_error} D:{defines:?} F:{flags:?} W:{warns:?} \
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
            merged.merge(&compile_params);
            if let Some(source_files) = &mut source_files {
                if source_files.contains(source_file) {
                    println!("Duplicate source file: {source_file}");
                } else {
                    source_files.insert(source_file.to_string());
                }
            }
        }
    }
    // Close the file
    drop(it);
    println!("{merged:#?}");
    if let Some(source_files) = &source_files {
        println!("{source_files:#?}");
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
