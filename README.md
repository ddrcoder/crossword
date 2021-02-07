# Introduction
This is a crossword puzzle generator/solver with a text UI, implemented in Rust.

# Design
`Crossword` models the set of possible solutions for a crossword puzzle of a given size. It consists of a `Dictionary`, a collection of `Line`s, woven together with `Cell`s.

## Components
Every `Cell` represents the set of letters which could be written in a given place in the crossword. Rebuses can be modeled a sequence of cells association with a single position on the rectangular board. Every `Cell` is associated with **two** `Line`s, one *across*, one *down*. More generally, any "weaving" of lines is possible including >2D puzzles.

A `Line` represents the set of words that could fill in a horizontal or vertical list of `Cell`s. Initially, all `Line`s contain the full set of words of the given length. Each `Line` maintains a histogram of letters at each position from the set of remaining words. E.g.: If a `Line` had a remaining set of words `{"cat", "car"}`, it would have 3 histograms of letters for the three positions, `[{'c': 2}, {'a': 2}, {'r': 1, 't': 1}]`.

Each `Cell` maintains the joint probability distribution of letters which could satisfy its position in both of its linked `Line`s. This is computed as the dot product of the two associated histograms.

## Pre-filtering
Solving could proceed at this point, but the set of possible words could be farther reduced by applying the fact that the distribution for each `Cell` may already have fewer letters than the distribution at each `Line`'s position. E.g.: An across `Line` may have many words with 'e' in the 3rd position, but all of those words should be eliminated if it crossed a down `Line` with no 'e's in the crossing position. This may lead to cascading effects, so such eliminations would be necessary to perform iteratively until no such conflicts remain.

## Solving
Puzzles are solved by constraint satisfaction, greedily by depth-first
search. At each depth, a cell with the fewest remaining choices is selected
arbitrarily, ignoring cells with 1 choise remaining. A letter is then
selecting by sampling from the distribution of letters for this cell. In
choosing this letter, the corresponding lines are then updated to eliminate
any words which contain a different letter at that position. The `Line`'s
histograms are then updated, as are the associated cells. If, at this point,
an affected `Cell`'s letter distribution becomes empty, this represents a
failure, and the search must **backtrack**. Otherwise, the search may
continue recursively. Additionally, if the selection of a letter reduces a
`Line`'s set of words to a single word, this word is added to the set of
committed words for the puzzle. If would introduce a duplicate word, this
represents another failure case and should be backtracked.

# Optimizations
Much of the time spent in the solver is spent reducing the remaining sets of words each line. These set operations are accelerated by use of an Inverted Index. Given the source dictionary, indices are constructed for the following sets:
* Words of length N.
* Words of length N with character C at position I.
* Words remaining for each `Line`.

To construct this, the dictionary is sorted (alphabetically, though this is arbitrary) into a list. Each word will be identified by its position in this list. The sets above are represented as sorted lists of these integer identifiers. The sets are constructed by simply iterating through the sorted word list once, appending the identifier of each word into the list corresponding to each of the above sets to which it belongs.

The `Line` indices are initially constructed from the set of words of the
target length. As letters are chosen, this set is intersected with the set of
words of that length matching that letter at that position. To reduce (but
not eliminate) the number of failures reached by choosing duplicate words,
the set of words chosen so far are also excluded in the same update
operation.

# Puzzle Layout
[Planned Work]
The intial construction of `Line`s and `Cell`s is subject to the dimensions of the puzzle and the presence of blank squares. First, `Cells` are constructed for all non-blank squares. Then, `Line`s are constructed for each valid start position, across and down. As part of the construction of each `Line`, it is linked with the `Cell` corresponding to each of its positions. To simplify borrow checking and lifetime management, all `Line`s and `Cell`s are identified by index.

In the current implementation, blank squares are not implemented. Instead, all puzzles are full rectangular grids.