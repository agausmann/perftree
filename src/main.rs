use std::collections::BTreeMap;
use std::env;
use std::io::{self, BufRead, BufReader, Write};
use std::process::{exit, Child, ChildStdin, ChildStdout, Command, Stdio};
use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};

const INITIAL_FEN: &str = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";

fn usage() -> ! {
    eprintln!("Usage: perftree <script>");
    exit(1);
}

fn main() -> io::Result<()> {
    let input = io::stdin();
    let input_handle = input.lock();
    let mut input_lines = input_handle.lines();
    let mut output = StandardStream::stdout(ColorChoice::Auto);

    let mut state = State::new(env::args().nth(1).unwrap_or_else(|| usage()))?;

    while let Some(line_result) = input_lines.next() {
        let line = line_result?;
        let mut words = line.split_whitespace();
        let cmd = match words.next() {
            Some(word) => word,
            None => continue,
        };

        match cmd {
            "fen" => {
                let fen = words.collect::<Vec<_>>().join(" ");
                if fen.is_empty() {
                    println!("{}", state.fen);
                } else {
                    state.fen(fen);
                }
            }
            "moves" => {
                let moves = words.map(|s| s.to_string()).collect::<Vec<_>>();
                if moves.is_empty() {
                    println!("{}", state.moves.join(" "));
                } else {
                    state.moves(moves);
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
                    state.depth(depth);
                } else {
                    println!("{}", state.depth);
                }
            }
            "root" => {
                state.root();
            }
            "parent" | "unmove" => {
                state.parent();
            }
            "child" | "move" => {
                if let Some(move_) = words.next() {
                    state.child(move_);
                } else {
                    eprintln!("missing argument, expected a child move");
                }
            }
            "diff" => match state.diff() {
                Ok(diff) => diff.write_colored(&mut output)?,
                Err(e) => eprintln!("cannot compute diff: {}", e),
            },
            "exit" | "quit" => {
                break;
            }
            other => {
                eprintln!("unknown command {:?}", other);
            }
        }
    }
    Ok(())
}

pub struct State {
    stockfish: Stockfish,
    script: Script,
    fen: String,
    moves: Vec<String>,
    depth: usize,
}

impl State {
    pub fn new<S>(cmd: S) -> io::Result<State>
    where
        S: Into<String>,
    {
        Ok(State {
            stockfish: Stockfish::new()?,
            script: Script::new(cmd),
            fen: INITIAL_FEN.to_string(),
            moves: Vec::new(),
            depth: 1,
        })
    }

    pub fn fen<S>(&mut self, fen: S)
    where
        S: Into<String>,
    {
        self.fen = fen.into();
        self.moves.clear();
    }

    pub fn moves<V>(&mut self, moves: V)
    where
        V: Into<Vec<String>>,
    {
        self.moves = moves.into();
    }

    pub fn depth(&mut self, depth: usize) {
        self.depth = depth;
    }

    pub fn root(&mut self) {
        self.moves.clear();
    }

    pub fn parent(&mut self) {
        self.moves.pop();
    }

    pub fn child<S>(&mut self, move_: S)
    where
        S: Into<String>,
    {
        self.moves.push(move_.into());
    }

    pub fn diff(&mut self) -> io::Result<Diff> {
        Ok(Diff::new(
            &self
                .script
                .perft(&self.fen, &self.moves, self.depth - self.moves.len())?,
            &self
                .stockfish
                .perft(&self.fen, &self.moves, self.depth - self.moves.len())?,
        ))
    }
}

#[derive(Debug, Clone)]
pub struct Diff {
    total_count: (u128, u128),
    child_count: BTreeMap<String, (Option<u128>, Option<u128>)>,
}

impl Diff {
    pub fn new(lhs: &Perft, rhs: &Perft) -> Diff {
        let mut child_count = BTreeMap::new();
        for (move_, &count) in &lhs.child_count {
            child_count.entry(move_.clone()).or_insert((None, None)).0 = Some(count);
        }
        for (move_, &count) in &rhs.child_count {
            child_count.entry(move_.clone()).or_insert((None, None)).1 = Some(count);
        }
        Diff {
            total_count: (lhs.total_count, rhs.total_count),
            child_count,
        }
    }

    pub fn write_colored<W>(&self, mut write: W) -> io::Result<()>
    where
        W: WriteColor,
    {
        let mut min_width = 0;
        for &(lhs, rhs) in self.child_count.values() {
            if let Some(lhs) = lhs {
                let digits = (lhs as f64).log10().ceil() as usize;
                min_width = min_width.max(digits);
            }
            if let Some(rhs) = rhs {
                let digits = (rhs as f64).log10().ceil() as usize;
                min_width = min_width.max(digits);
            }
        }

        for (move_, &(lhs, rhs)) in &self.child_count {
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
        let (lhs, rhs) = self.total_count;
        if lhs != rhs {
            write.set_color(ColorSpec::new().set_fg(Some(Color::Red)).set_bold(true))?;
        }
        write!(write, "total  {}  {}", lhs, rhs)?;
        write.reset()?;
        writeln!(write)?;

        Ok(())
    }
}

pub trait Engine {
    fn perft(&mut self, fen: &str, moves: &[String], depth: usize) -> io::Result<Perft>;
}

pub struct Perft {
    total_count: u128,
    child_count: BTreeMap<String, u128>,
}

pub struct Script {
    cmd: String,
}

impl Script {
    pub fn new<S>(cmd: S) -> Script
    where
        S: Into<String>,
    {
        Script { cmd: cmd.into() }
    }
}

impl Engine for Script {
    fn perft(&mut self, fen: &str, moves: &[String], depth: usize) -> io::Result<Perft> {
        let mut command = Command::new(&self.cmd);
        command.arg(depth.to_string());
        command.arg(fen);
        if !moves.is_empty() {
            command.arg(moves.join(" "));
        }

        let output = command.output()?;
        //re-raise output from stderr
        io::stderr().write_all(&output.stderr)?;
        let mut lines = output.stdout.lines();

        let mut child_count = BTreeMap::new();
        loop {
            let line = lines
                .next()
                .ok_or(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "unexpected eof while parsing script output",
                ))
                .and_then(|result| result)?;

            if line.is_empty() {
                break;
            }
            let mut parts = line.split_whitespace();
            let move_ = parts
                .next()
                .ok_or(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "unexpected end of line; expected move and count separated by spaces",
                ))?
                .to_string();
            let count = parts
                .next()
                .ok_or(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "unexpected end of line; expected move and count separated by spaces",
                ))?
                .parse()
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e))?;
            child_count.insert(move_, count);
        }

        let total_count = lines
            .next()
            .ok_or(io::Error::new(
                io::ErrorKind::InvalidInput,
                "unexpected eof while parsing script output",
            ))
            .and_then(|result| result)?
            .parse()
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e))?;

        Ok(Perft {
            child_count,
            total_count,
        })
    }
}

pub struct Stockfish {
    child: Child,
    inp: BufReader<ChildStdout>,
    out: ChildStdin,
}

impl Stockfish {
    pub fn new() -> io::Result<Stockfish> {
        let mut child = Command::new("stockfish")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()?;

        let mut inp = BufReader::new(child.stdout.take().expect("stdout not captured"));
        // consume/skip header
        let mut buf = String::new();
        inp.read_line(&mut buf)?;

        let out = child.stdin.take().expect("stdin not captured");

        Ok(Stockfish { child, inp, out })
    }
}

impl Engine for Stockfish {
    fn perft(&mut self, fen: &str, moves: &[String], depth: usize) -> io::Result<Perft> {
        // send command to stockfish
        write!(self.out, "position fen {}", fen)?;
        if !moves.is_empty() {
            write!(self.out, " moves {}", moves.join(" "))?;
        }
        write!(self.out, "\ngo perft {}\n", depth)?;

        let mut buf = String::new();

        // parse child counts
        let mut child_count = BTreeMap::new();
        loop {
            buf.clear();
            self.inp.read_line(&mut buf)?;
            if buf.trim().is_empty() {
                break;
            }
            let mut parts = buf.trim().split(": ");
            let move_ = parts
                .next()
                .ok_or(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "unexpected end of line",
                ))?
                .to_string();
            let count = parts
                .next()
                .ok_or(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "unexpected end of line",
                ))?
                .parse()
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e))?;
            child_count.insert(move_, count);
        }

        // parse total count
        buf.clear();
        self.inp.read_line(&mut buf)?;
        let mut parts = buf.trim().split(": ");
        let total_count = parts
            .nth(1)
            .ok_or(io::Error::new(
                io::ErrorKind::InvalidInput,
                "unexpected end of line",
            ))?
            .parse()
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e))?;

        // throw away empty line
        buf.clear();
        self.inp.read_line(&mut buf)?;

        Ok(Perft {
            child_count,
            total_count,
        })
    }
}

impl Drop for Stockfish {
    fn drop(&mut self) {
        let _ = self.child.kill();
    }
}
