# Isobar Core

This directory containts the native core of application as a pure Rust library that is agnostic to the details of the underlying platform. It is a dependency of the sibling `isobar_server` crate, which provides it with network and file I/O as well as the ability to spawn futures in the foreground and on background threads.
