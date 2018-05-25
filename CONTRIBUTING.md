# Contributing to Isobar

This project is still in the very early days, and isn't going to be usable for even basic editing for some time. At this point, we're looking for contributors that are willing to roll up their sleeves and solve problems. Please communicate with us however it makes sense, but in general opening a *pull request that fixes an issue* is going to be far more valuable than just reporting an issues.

As the architecture stabilizes and the surface area of the project expands, there will be increasing opportunities to help out. To get some ideas for specific projects that could help in the short term, check out [issues that are labeled "help wanted"](https://github.com/siberianmh/isobar/issues?q=is%3Aopen+is%3Aissue+label%3A%22help+wanted%22). If you have an idea you'd like to pursue outside of these, that's awesome, but you may want to discss it with us in an issue first to ensure it fits before spending too much time on it.

It's really important to us to have a smooth on-ramp for contributors, and one great way you can contribute is by helping us improve this guide. If your experience is bumpy, can you open a pull request that makes it smoother for the next person?

## Communicating with maintainers

The best way to communicate with maintainers is by posting issue to this repository. The more thought you put into articulating your question or idea, the more value you'll be adding the community and easier it will be for maintainers to respond. That said, just try your best. If you have something you want to say, we'd prefer that you say it imperfectly rather than not saying it at all.

You can also communicate with maintainers or other community members on the `#isobar` channel on Siberian Media Holding's public slack instance. After you [request an invite via this form](https://siberianmh-slack.herokuapp.com/), you can access our Slack instance at https://siberian-holding.slack.com.

## Building

So far, we have only built this project on macOS. If you'd like to help improve our build or documnetation to support other platfroms, that would be a huge help!

### Install system dependencies

#### Install Node v8.9.3

To install Node, you can intsall [`nvm`](https://github.com/creationix/nvm) and then run `nvm install v8.9.3`.

Later version may work, but you should ideally run the build with the same version of Node that is bundled into Isobar's current Electron dependency. If in doubt, you can check the version of the `electron` dependency in [`isobar_electron/package.json`](https://github.com/siberianmh/isobar/blob/master/isobar_electron/package.json), then run `process.versions.node` in the console of that version of Electron to ensure that these instructions haven't gotten out of date.

#### Install Rust

You can install Rust via [`rustup`](https://www.rustup.rs/). We currently require building on the nightly channel in order to use `wasm_bindgen` for browser support.

### Run the build script

This repository contains several components in top-level folders prefixed with `isobar_*`. To build all of the components, simply run this in the root of the repository:

```sh
script/build
```

To build a release version (which will be much faster):

```
script/build --release
```

## Running

We currently *only* support launching the application via the CLI. For this to work, you need to set `ISOBAR_SRC_PATH` environment variable to the location of your repository. The CLI also currently *requires* an argument:

```sh
ISOBAR_SRC_PATH=. script/isobar .
```

That assumes you built with `--release`. To run the debug version, use `isobar_debug` instead:

```sh
ISOBAR_SRC_PATH=. script/isobar_debug .
```

### Running tests and benchmarks

* All tests: `script/test`
* Rust tests: `cargo test` in the root of the repository or a Rust subfolder.
* Front-end tests: `cd isobar_electron && npm test`
* Benchmarks: `cargo bench`
