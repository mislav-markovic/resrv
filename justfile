alias b := build
alias r := serve
alias d := debug

default_serve_dir := "./assets"
default_serve_url := "localhost:3001"


# build project
build:
  cargo build

# start the resrv that will serve assets
serve serve_dir=default_serve_dir serve_url=default_serve_url: build
  cargo run --bin resrv -- --dir {{serve_dir}} --url {{serve_url}}

# start resrv in debug mode
debug serve_dir=default_serve_dir serve_url=default_serve_url:
  RUST_LOG=debug cargo run --bin resrv -- --dir {{serve_dir}} --url {{serve_url}}

# start resrv for dev mode with dir pointing to `./assets/` and serving on `localhost:3001`
dev: (debug "./assets" "localhost:3001")

# start dev resrv that rebuilds and restarts on change
watch:
  bacon reload-dev
