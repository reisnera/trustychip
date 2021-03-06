# TrustyChip

A Chip-8 libretro emulator core written in Rust.

## Building

I am developing this on the latest stable rustc, so you probably should use that too.

To build:

```shell
git clone --recurse-submodules https://github.com/reisnera/trustychip.git
cd trustychip
cargo build (or cargo build --release)
```

You will then find the built shared library somewhere in the `target` directory.
Simply load that library using a libretro frontend and you're (allegedly) all set!

## Building documentation

Project documentation can be built with the following:

```shell
cargo doc --no-deps --document-private-items
```

Obviously, you can omit `--no-deps` if you wish to build dependency docs as
well. Documentation can then be found in .\target\doc\trustychip\index.html
