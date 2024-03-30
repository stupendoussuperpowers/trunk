# trunk

tail but filter when following.

# Why?

I need a small tool for work that would make it easy to filter `tail -f` outputs live. Decided to put one together.

Has only been tested on Windows so far.

TODO: ensuring linux support as well.

# Usage

Added feature for "sieving". Will follow for file changes

`trunk -s <filter> /path/to/file`

Usual tail commands are compatible

`trunk -n <number of lines> /path/to/file`

`trunk -f /path/to/file`

.

.

.

`trunk -h`

```
Usage: trunk.exe [OPTIONS] <FILE>

Arguments:
<FILE> Path of the file to tail/follow

Options:
-f, --follow Follow a file for live changes
-s, --sieve <SIEVE> Phrase to filter new lines with. Will automatically enable [-f --follow] [default: ]
-n, --num-lines <NUM_LINES> Number of lines from the end to tail [default: 5]
-h, --help Print help
-V, --version Print version
```

# Building

`cargo build`
