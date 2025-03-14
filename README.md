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

Install the `perftree` CLI application from the crates.io repository with
`cargo`:

```
cargo install perftree-cli
```

## Usage

### Your perft script

`perftree` requires some way to execute the `perft` function on your chess
engine. Currently, it uses an executable file (e.g. a script) provided by the
user, which will be invoked like this:

```
./your-perft.sh "$depth" "$fen" "$moves"
```

where

- `$depth` is the depth parameter of the perft function.

- `$fen` is the [Forsyth-Edwards Notation][fen] string of a "base" position
  (set by the `fen` command)

- `$moves` is a space-separated sequence of moves starting from the base position,
  as set by the `move`/`unmove`/`moves` commands.  
  The moves are formatted in UCI notation: `$source$target$promotion`, where castling
  is encoded as the king's move during the castle. Examples: `e2e4`, `e1g1`, `a7b8Q`.  
  If a `$moves` argument is not provided, then no moves have to be applied.

Your engine should apply the `$moves` starting from the position specified by `$fen`.
The position reached after applying `$moves` is referred to as the "current" position.
At that position, it should execute perft with the given `$depth`. A depth of 0 would
just count the current position, so a total of 1. A depth of 1 counts all moves available
at this position, depth 2 counts all moves available after each "depth-1" move, and so on.

Then, output the results as follows:

- For each move that is possible at the current position, print the move in UCI notation,
followed by a space, then the number of states counted by perft that start with that move.
(This is each recursive case from the current position; i.e. for each available move at the
current position, make the move and call perft with a depth of `$depth - 1`)

- After the list of moves, print a blank line.

- Finally, print the total perft result for the current position on its own line.

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

### Running perftree

Run `perftree` from the commandline, and pass the path to your perft executable
as the first argument:

```bash
perftree ./your-script.sh
```

`perftree` understands the following commands:

- `fen [new_fen]` - Set the FEN string of the root node, and clears the move
  list. When `new_fen` is not provided, the current FEN string will be printed
instead.

- `moves [new_moves ...]` - Set the move list. When `new_moves` is not
  provided, the current move list will be printed instead.

- `depth [new_depth]` - Set the max depth for `perft`. When `new_depth` is not
  provided, the current depth will be printed instead.

- `root` - Clears the move list, effectively changing to the root node of the
  game tree.

- `child|move <move>` - Pushes the given move onto the move list, effectively
  changing to the child node identified by the given move in the current state.

- `parent|unmove` - Pops a move from the move list, effectively changing to the
  parent node of the current state.

- `diff` - Calculates and outputs a diff of the `perft` results for the current
  node. Your results will be on the left, and Stockfish will be on the right.
A missing number means that the move did not exist in the output from the
corresponding engine.

- `exit|quit` - Exits the program.

### Example

[![asciicast](https://asciinema.org/a/rWP6zJFUA3ZldASxfHWv3iL5z.svg)](https://asciinema.org/a/rWP6zJFUA3ZldASxfHWv3iL5z)

[perft]: https://www.chessprogramming.org/Perft
[stockfish]: https://stockfishchess.org/
[fen]: https://www.chessprogramming.org/Forsyth-Edwards_Notation
