# Isobar Electron

This is the front-end of the desktop application. It spawns an instance of `isobar_server`, where the majority of application logic resides, and communicates with it over a domain socket.

## Building and running

This assumes `isobar_electron` is cloned as part of the Isobar repository and that all of its sibling pacakges are next to it. Also, make sure you have installed the required [system dependencies](../CONTRIBUTING.md#install-system-dependencies) before proceeding.

```sh
# Move to this subdirectory of the repository:
cd isobar_electron

# Install and build dependencies:
npm install

# Launch Electron:
npm start
```
