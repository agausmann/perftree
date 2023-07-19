use std::collections::BTreeMap;
use std::io::{self, BufRead, BufReader, Write};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};

const INITIAL_FEN: &str = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";

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

    pub fn fen(&self) -> &str {
        &self.fen
    }

    pub fn set_fen<S>(&mut self, fen: S)
    where
        S: Into<String>,
    {
        self.fen = fen.into();
        self.moves.clear();
    }

    pub fn moves(&self) -> &[String] {
        &self.moves
    }

    pub fn set_moves<V>(&mut self, moves: V)
    where
        V: Into<Vec<String>>,
    {
        self.moves = moves.into();
    }

    pub fn depth(&self) -> usize {
        self.depth
    }

    pub fn set_depth(&mut self, depth: usize) {
        self.depth = depth;
    }

    pub fn goto_root(&mut self) {
        self.moves.clear();
    }

    pub fn goto_parent(&mut self) {
        self.moves.pop();
    }

    pub fn goto_child<S>(&mut self, move_: S)
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

    pub fn set_chess960(&mut self, chess960: bool) {
        self.stockfish.chess960 = chess960;
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

    pub fn total_count(&self) -> (u128, u128) {
        self.total_count
    }

    pub fn child_count(&self) -> &BTreeMap<String, (Option<u128>, Option<u128>)> {
        &self.child_count
    }
}

pub trait Engine {
    fn perft(&mut self, fen: &str, moves: &[String], depth: usize) -> io::Result<Perft>;
}

pub struct Perft {
    total_count: u128,
    child_count: BTreeMap<String, u128>,
}

impl Perft {
    pub fn new(total_count: u128, child_count: BTreeMap<String, u128>) -> Perft {
        Perft {
            total_count,
            child_count,
        }
    }

    pub fn total_count(&self) -> u128 {
        self.total_count
    }

    pub fn child_count(&self) -> &BTreeMap<String, u128> {
        &self.child_count
    }
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
                .ok_or_else(|| {
                    io::Error::new(
                        io::ErrorKind::InvalidInput,
                        "unexpected eof while parsing script output",
                    )
                })
                .and_then(|result| result)?;

            if line.is_empty() {
                break;
            }
            let mut parts = line.split_whitespace();
            let move_ = parts
                .next()
                .ok_or_else(|| {
                    io::Error::new(
                        io::ErrorKind::InvalidInput,
                        "unexpected end of line; expected move and count separated by spaces",
                    )
                })?
                .to_string();
            let count = parts
                .next()
                .ok_or_else(|| {
                    io::Error::new(
                        io::ErrorKind::InvalidInput,
                        "unexpected end of line; expected move and count separated by spaces",
                    )
                })?
                .parse()
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e))?;
            child_count.insert(move_, count);
        }

        let total_count = lines
            .next()
            .ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "unexpected eof while parsing script output",
                )
            })
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
    chess960: bool,
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

        Ok(Stockfish {
            child,
            inp,
            out,
            chess960: false,
        })
    }
}

impl Engine for Stockfish {
    fn perft(&mut self, fen: &str, moves: &[String], depth: usize) -> io::Result<Perft> {
        // Enable/disable Chess960
        write!(
            self.out,
            "setoption name UCI_Chess960 value {}",
            self.chess960
        )?;
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
                .ok_or_else(|| {
                    io::Error::new(io::ErrorKind::InvalidInput, "unexpected end of line")
                })?
                .to_string();
            let count = parts
                .next()
                .ok_or_else(|| {
                    io::Error::new(io::ErrorKind::InvalidInput, "unexpected end of line")
                })?
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
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "unexpected end of line"))?
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
