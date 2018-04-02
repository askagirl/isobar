# Isobar Core Node Bindings

This subproject provides an interface to the `isobar_core` library from JavaScript. It builds a shared library which is designed to be loaded as a Node.js complied add-on.

## Building

This project depends on the [`napi`](https://github.com/siberianmh/napi) crate, which provides a safe interface to Node's N-API. It depends on the tandem Node.js pacakge `napi` to provide a build harness. Running `npm build` will invoke that build hardness which sets up the environment variables and linker falgs for Cargo.

Currently, `napi` is expected to be present as a sibling of the `isobar` repository until I take the time to set it up more correctly.
