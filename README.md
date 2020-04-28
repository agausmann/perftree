# perftree

A `perft` debugger. Compare your chess engine to Stockfish and quickly find
discrepancies in move generation.

## How it works

When debugging a chess engine, it is common to compare its move generation to a
known-good engine using the results of the [`perft`][perft] function, which
counts all of the nodes at some given depth from some starting position.  Using
these results, one can quickly isolate the problematic subtrees and figure out
where the move generation differs between the two engines.

Instead of comparing the results and walking the tree manually, I use and
maintain `perftree`, a semi-automatic debugger that does that hard work for
you. It can keep track of where you are in the game tree, evaluate the `perft`
function at the current position, and compare the results automatically,
highlighting the differences so they are easy to pick out.

## Install

`perftree` uses Stockfish, a well-known engine used widely throughout the chess
community, as a trusted source of perft results. Download and install
[Stockfish][stockfish] if you haven't already, and make sure you can run it
from the command line with the command `stockfish`.

Install the `perftree` program from the crates.io repository with `cargo`:

```
cargo install perftree
```

## Usage

`perftree` requires some way to invoke the `perft` function on your chess
engine. Currently, it expects the user to provide a script, which will be
invoked like this:

```
./your-perft.sh "$depth" "$fen" "$moves"
```

where

- `$depth` is the maximum depth of the evaluation,

- `$fen` is the [Forsyth-Edwards Notation][fen] string of some base position,

- `$moves` is an optional space-separated list of moves from the base position
  to the position to be evaluated, where each move is formatted as
`$source$target$promotion`, e.g. `e2e4` or `a7b8Q`.

The script is expected to output the results of the perft function to standard
output, with the following format:

- For each move available at the current position, print the move and the
  number of nodes at the given depth which are an ancestor of that move,
separated by whitespace.

- After the list of moves, print a blank line.

- Finally, print the total node count on its own line.

For example, this is what the depth-3 perft of the starting position should look
like:

```
$ ./your-perft.sh 3 "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1"
a2a3 380
b2b3 420
c2c3 420
d2d3 539
e2e3 599
f2f3 380
g2g3 420
h2h3 380
a2a4 420
b2b4 421
c2c4 441
d2d4 560
e2e4 600
f2f4 401
g2g4 421
h2h4 420
b1c3 440
g1h3 400
b1a3 400
g1f3 440

8902
```

## Example

[![asciicast](https://asciinema.org/a/rWP6zJFUA3ZldASxfHWv3iL5z.svg)](https://asciinema.org/a/rWP6zJFUA3ZldASxfHWv3iL5z)

[perft]: https://www.chessprogramming.org/Perft
[stockfish]: https://stockfishchess.org/
[fen]: https://www.chessprogramming.org/Forsyth-Edwards_Notation
