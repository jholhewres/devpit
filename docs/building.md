# Building and running devpit

## Building it

- Rust 1.93+, Node 22+, pnpm
- the WebKitGTK development packages:

```sh
sudo apt install libwebkit2gtk-4.1-dev libgtk-3-dev libsoup-3.0-dev \
                 build-essential curl file libssl-dev libayatana-appindicator3-dev
```

## Running it

```sh
make setup    # dependencies
make dev      # the app, with hot reload
make test     # guards, Rust tests, frontend tests
make build    # release bundle, frontend included
```

`make` on its own lists every target.

Repository rules, guards and conventions are in [AGENTS.md](../AGENTS.md).
