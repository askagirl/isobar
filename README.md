# Isobar

This is an *experiment* (that may go nowhere) in a new design for an Electron-based text editor wihhc intends to expore the following ideas:

## Prefer complied code

The core of the application is written in Rust. The core is a pure Rust library that lives in `isobar_core`. The core is then exposed to JavaScript as a native add-on for Node.js in `isobar_core_node`.

## Store text in a CRDT and maximize parallelism

The buffer representation builds on what we learned with the Teletype CRDT, representing the document as a sequence of fragments in a persistent copy-on-write B+ tree. This should have a number of benefits.

* **No need for a marker index.** Rather than maintaining the positions of markers on every edit, we can use abstract positions to maintain stable references into the text. Abstrcat positions can be stored in an interval tree for cases where we need to perfrom efficient stabbing queries, but such a tree would not need to be modified on every edit.
* **Background packages.** CRDTs allow multiple threads to concurrently modify a buffer, which should make it possible to give packages synchronous write access to buffers even if they are running on background threads.
* **Maximal parallelism** The copy-on-write data-structure makes cloning a buffer a constant-time operation. The CRDT means that abstract positions computed on a background thread remain valid when moved to the main thread, even if the buffer has changed in the meantime.

## Prioritize performance, subjugate web technology

Atom embraces web technology. Isobar aims to subjugate it. While we still plan to give pacakge authors maximal freedom to exploit all features of the web-based environment, the DOM will be treated more as an implementation detail rather than as a foundational abstraction. We aim to minimize leakage of any DOM-related concepts into our public APIs, with the possible exception of CSS for theming.

We want to avoid the DOM for rendering the text editor, for example, insted using a canvas, WebGL, or any other approach that can give us extremely good performance. This may make syntax themes slightly more difficult to build, and we accept that trade-off.

Focus on minimizing startup time from the beginning and don't add *anything* that degrades it.

## Static typing, versioning, and minimalism

Version APIs wherever possible to allow evolution without breakage. Use static typing wherever possible to clarify interfaces, improve documentation, and minimize the chances of inadvertent breakage. In core, expose the *minimal* API needed to get the job done, pushing other concerns out to libraries.

# Informal Short-Term Roadmap

This is mostly for our use right now and might not make a ton of sense just yet to the casual observer.

* [x] A CRDT-backed buffer implementation with the ability to edit text
* [x] Bindings to N-API enabling a buffer's text to be rtrieved as a JS string and edits to be performed.
* [x] Refactor N-API bindings to use marker types in N-API values.
* [ ] Covalent: Error handling
* [ ] Covalent: Async promise resolution
* [ ] Implement `BroadcastLatest`, a single-producer/multi-consumer broadcast channel that implements the `futures::Stream` trait and returns the most recently assigned value when polled. This will be used to propagate buffer and editor changes to observers.
* [ ] Implement a read-only editor view in Electron using canvas. How fast can we get it on screen? How fast can we scroll it?
* [ ] Implement syntax highlighting with tree-sitter and a simple custom theming system. How fast can we load and scroll now?
* [ ] Add selections/cursors and multi-cursor editing. How many cursors can we comfortably type with?
* [ ] Create a server process to load and save buffers and incrementally persist their edit histories to a database.
* [ ] Multi-tab, single-pane workspace. Let's try going all-in on React. It's a big dependency, but we can snapshot it.
* [ ] Make sure right-to-left text works
* [ ] Soft wraps and folds
* [ ] File finder backed by a bitmap index

At this point we should have a very minimal working text editor that's obviously missing a ton of important features. There's a very long tail of features that will be required to make an even minimally usable system. If we decide to continue the experiment, here's a list of major features we'll need to implement in rough priority order. Each of these bullets obviously implies a big task list of its own.

* Windows and Linux support
* Key bindings and command system - We need to come up with something that's not based on seletors or DOM events.
* Smart indent - We can use the syntax tree and a language definition file.
* Autocomplete - Let's try to index a default subsequence match provider and support the language server protocol early on.
* Find and replace - Let's base this on `ripgrep` from the get-go. We can add a slower fallback implementation based on `pcre` later for more complex regular expressions.
* Status bar
* Project browser (tree view)
* Snippets
* Multiple panes
* Symbol navigator based on tree sitter
* Savable projects and workspaces
* Integrate existing Git package
