# Central repository for work on libp2p

[![dependency status](https://deps.rs/repo/github/libp2p/rust-libp2p/status.svg)](https://deps.rs/repo/github/libp2p/rust-libp2p)

This repository is the central place for Rust development of the [libp2p](https://libp2p.io) spec.

**This readme will be more fleshed out the closer the project gets to completion.
Right now everything including the crate organization is very much Work in Progress.**

## Documentation

This repository includes a façade crate named `libp2p`, which reexports the rest of the repository.

For documentation, you are encouraged to clone this repository or add `libp2p` as a dependency in
your Cargo.toml and run `cargo doc`.

```toml
[dependencies]
libp2p = { git = "https://github.com/libp2p/rust-libp2p" }
```

## Notable users

(open a pull request if you want your project to be added here)

- https://github.com/paritytech/polkadot
- https://github.com/paritytech/substrate
