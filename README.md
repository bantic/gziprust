[![Build Status](https://travis-ci.org/bantic/gziprust.svg?branch=master)](https://travis-ci.org/bantic/gziprust)

# Gzip in Rust

A Gzip decoder (inflater) written entirely in Rust, with (almost) no crates.

## TODOS

- Better command-line argument parsing (allow debug output or simply decoding to a file)
- Test multiple blocks
- Allow matches to cross backwards into previous blocks
- Add support for uncompressed blocks
- try it out via WebAssembly?
- Add a playback mode that shows a bit more about the compression -- print out at a given pace, each thing decoded...
  So the same amount of time to print a single decoded literal as to print out a matched chunk...
- Update the playback mode to also vary the pace by how many bits were required to encode the literal
- Update playback mode to show some visual indication of both bits-per-literal (different sizes? or just shades of a color) and matched lengths (use a color + perhaps a dist,len annotation)
- Make it a command-line binary
- better name
- perf profiling

## Done

- add method to advance bit iterator to next byte, and to turn it back into a byte iterator, to do checksum matching.
  Didn't add the method, but did add code to confirm the size and crc32 matched
