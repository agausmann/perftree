use perftree::{Diff, State};
use std::env;
use std::io::{self, BufRead, Write};
use std::process::exit;
use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};

fn usage() -> ! {
    eprintln!("Usage: perftree <script>");
    exit(1);
}

struct Prompt<R> {
    lines: std::io::Lines<R>,
}

impl<R> Prompt<R>
where
    R: BufRead,
{
    fn new(buf_read: R) -> Self {
        Self {
            lines: buf_read.lines(),
        }
    }

    fn prompt(&mut self, ps: &str) -> io::Result<Option<String>> {
        if atty::is(atty::Stream::Stdin) {
            if atty::is(atty::Stream::Stdout) {
                print!("{}", ps);
                io::stdout().flush()?;
            } else if atty::is(atty::Stream::Stderr) {
                eprint!("{}", ps);
                io::stderr().flush()?;
            }
        }
        self.lines.next().transpose()
    }
}

fn main() -> io::Result<()> {
    let input = io::stdin();
    let mut prompt = Prompt::new(input.lock());
    let mut output = StandardStream::stdout(ColorChoice::Auto);

    let mut state = State::new(env::args().nth(1).unwrap_or_else(|| usage()))?;

    while let Some(line) = prompt.prompt("> ")? {
        let mut words = line.split_whitespace();
        let cmd = match words.next() {
            Some(word) => word,
            None => continue,
        };

        match cmd {
            "fen" => {
                let fen = words.collect::<Vec<_>>().join(" ");
                if fen.is_empty() {
                    println!("{}", state.fen());
                } else {
                    state.set_fen(fen);
                }
            }
            "moves" => {
                let moves = words.map(|s| s.to_string()).collect::<Vec<_>>();
                if moves.is_empty() {
                    println!("{}", state.moves().join(" "));
                } else {
                    state.set_moves(moves);
                }
            }
            "depth" => {
                if let Some(depth) = words.next() {
                    let depth = match depth.parse() {
                        Ok(x) => x,
                        Err(e) => {
                            eprintln!("cannot parse given depth: {}", e);
                            continue;
                        }
                    };
                    state.set_depth(depth);
                } else {
                    println!("{}", state.depth());
                }
            }
            "root" => {
                state.goto_root();
            }
            "parent" | "unmove" => {
                state.goto_parent();
            }
            "child" | "move" => {
                if let Some(move_) = words.next() {
                    state.goto_child(move_);
                } else {
                    eprintln!("missing argument, expected a child move");
                }
            }
            "diff" => match state.diff() {
                Ok(diff) => write_colored(&diff, &mut output)?,
                Err(e) => eprintln!("cannot compute diff: {}", e),
            },
            "exit" | "quit" => {
                break;
            }
            "chess960" => {
                state.set_chess960(true);
            }
            "nochess960" => {
                state.set_chess960(false);
            }
            other => {
                eprintln!("unknown command {:?}", other);
            }
        }
    }
    Ok(())
}

pub fn write_colored<W>(diff: &Diff, mut write: W) -> io::Result<()>
where
    W: WriteColor,
{
    let mut min_width = 0;
    for &(lhs, rhs) in diff.child_count().values() {
        if let Some(lhs) = lhs {
            let digits = (lhs as f64).log10().ceil().max(0.0) as usize;
            min_width = min_width.max(digits);
        }
        if let Some(rhs) = rhs {
            let digits = (rhs as f64).log10().ceil().max(0.0) as usize;
            min_width = min_width.max(digits);
        }
    }

    for (move_, &(lhs, rhs)) in diff.child_count() {
        if lhs != rhs {
            write.set_color(ColorSpec::new().set_fg(Some(Color::Red)).set_bold(true))?;
        }
        write!(write, "{}", move_)?;
        if let Some(lhs) = lhs {
            write!(write, "  {:>width$}", lhs, width = min_width)?;
        } else {
            write!(write, "  {:>width$}", "", width = min_width)?;
        }
        if let Some(rhs) = rhs {
            write!(write, "  {:>width$}", rhs, width = min_width)?;
        } else {
            write!(write, "  {:>width$}", "", width = min_width)?;
        }
        writeln!(write)?;
        write.reset()?;
    }

    writeln!(write)?;
    let (lhs, rhs) = diff.total_count();
    if lhs != rhs {
        write.set_color(ColorSpec::new().set_fg(Some(Color::Red)).set_bold(true))?;
    }
    write!(write, "total  {}  {}", lhs, rhs)?;
    write.reset()?;
    writeln!(write)?;

    Ok(())
}
