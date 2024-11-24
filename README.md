# resrv

Intended for usage during development of web sites. Serves directory of assets and enables live reload of documents linked from `html` page.

## Installation

### Prerequisite

You need to have git installed as well as recent rust version (including cargo tool)

### Steps

Run following commands:

```sh
git clone https://github.com/mislav-markovic/resrv.git
cd resrv
cargo install --path .
```

## Usage

To start serving files in `./assets` directory on port `3001`:

```sh
resrv --dir assets/ --url localhost:3001
```

`--url` is optional, defaults to `localhost:9812`

## Build

Project provides recipes for [just](https://github.com/casey/just) task runner. If you have just locally then just run 

```sh
just build
```

in project root.

Without `just`, running `cargo build` directly also works.

## How it works

All files within `--dir` directory are served by `resrv` server. For routes that end in '/' `index.html` is appended and served.
For every requested `.html` file, `script` block is injected that establishes websocket connection to server. On this connection events that trigger reload will be received.
Server reacts on file change, content of files is currenlty not taken into consideration, any filesystem event that possaibly changes file data will trigger reload.
