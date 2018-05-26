# Iosbar Browser

This directory packages Isobar for use in a web browser. Beacuse browser don't provide access to the underlying system, when running in a browser, Isobar depends on being able to connect to a shared workspace on a remote instance of the `isobar_server` executable. This directory contains a [development web server](./script/server) that serves a browser-compatible user itnerface and proxies connections to `isobar_server` over WebSockets.

Assuming you have built Isobar with `script/build --release` in the root of this repo, you can present a web-based UI for any Isobar instance as follows.

* Start an instance `isobar_server` listening for incoming connections on a port 8080:
  ```sh
  # Run in the root of the repository (--headless is optional)
  script/isobar --listen=8080 --headless your_project_dir
  ```
* Start the development web server:
  ```sh
  isobar_browser/script/server
  ```
