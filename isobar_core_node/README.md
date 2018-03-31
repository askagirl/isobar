# Isobar Core Node Bindings

This subproject provides an interface to the `isobar_core` library from JavaScript. It builds a shared library which is designed to be loaded as a Node.js complied add-on.

## Building

Because the target library looks up symbols from Node dynamically, it cannot be build with cargo directly without additional linker flags. See `scripts/build.js` for details.

This project depends on the [`collider`](https://github.com/siberianmh/cillider) crate, (which provides a safe interface to Node's N-API. Currently, `collider` is expected to be present as a sibling of the `isobar` repository until I take the time to set it up more correctly.
