# Contributing to Isobar

## Building

So far, we have only built this project on macOS. If you'd like to help improve our build or documnetation to support other platfroms, that would be a huge help!

### Install system dependencies

#### Install Node v8.9.3

To install Node, you can intsall [`nvm`](https://github.com/creationix/nvm) and then run `nvm install v8.9.3`.

Later version may work, but you should ideally run the build with the same version of Node that is bundled into Isobar's current Electron dependency. If in doubt, you can check the version of the `electron` dependency in [`isobar_electron/package.json`](https://github.com/siberianmh/isobar/blob/master/isobar_electron/package.json), then run `process.versions.node` in the console of that version of Electron to ensure that these instructions haven't gotten out of date.

#### Install Rust

You can install Rust via [`rustup`](https://www.rustup.rs/). We currently build correctly on Rust 1.24.1, but frequently build on the nightly channel in development to enable formatting of generated bindings. The nightly channel should not be *required* however, and if it is, that's a bug.

### Build the Electron app

This repository cntains several components in top-level folders prefixed with `isobar_*`. The main application is located in `isobar_electron`, and you can build it as follows:

```sh
# Move to this subdirectory of the repository:
cd isobar_electron

# Install and build dependencies:
npm install

# Launch Electron:
npm start
```

If you want to *rebuild* the Rust dependencies after making changes and test them in the Electron app, run this

```sh
# Rebuild Rust dependencies:
npm rebuild isobar
```

### Build other modules independently

If you're working on a particular subsystem, such as [`isobar_core`](./isobar_core), you can build and test it independently of the Electron app. Each top-level module should have its own instructions in its README.
