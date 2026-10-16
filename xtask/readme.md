<https://docs.rs/xtasks/latest/xtasks/>

Run from the workspace root (`.cargo/config.toml` defines the `cargo xtask` alias).

## `cargo xtask uf2 [--debug] [-o PATH]`

Build `ropk-fw`, objcopy to `.bin`, convert to `.uf2` for the nice!nano bootloader.

## `cargo xtask decode [--raw] [--grep TEXT] [--file PATH]... [HEX...]`

Decode radio payloads on the host with `ropk-proto`, no board needed.

```bash
# one payload ([rtype][body..] = ScRadioData.payload)
cargo xtask decode f1 1e 06 45 07 01 00 00 00 ...

# paste straight from a defmt log line
cargo xtask decode "12.004 INFO  Payload  : [f1, 1e, 06, 45, ...]"

# every *.hex in a folder (or a single file)
cargo xtask decode --file crates/ropk-proto/tests/fixtures/good

# a saved defmt log, only the Payload lines
cargo xtask decode --grep Payload < boot.log

# a raw radio RAM dump with the [LEN][S1] header still on (e.g. C rfrx prints)
cargo xtask decode --raw 03 02 f2 00 00
```

Exits non-zero if any line fails to parse or decode.

Captured payloads worth keeping go in `crates/ropk-proto/tests/fixtures/{good,bad}/*.hex`
(one per line, `#` comments, optional `#! inputs N` before a payload). They're
replayed by `cargo test -p ropk-proto --test fixtures`.
