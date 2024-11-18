alias b := build
alias r := serve
alias d := debug


# build project
build:
  cargo build

# start the server that will serve assets
serve: build
  cargo run --bin server

# start server in debug mode
debug:
  RUST_LOG=debug cargo run --bin server

# start server for dev mode with dir pointing to `./assets/` and serving on `localhost:3001`
dev:
  RUST_LOG=debug cargo run --bin server -- --dir "./assets/" --url "localhost:3001"

# start dev server that rebuilds and restarts on change
watch:
  bacon reload-dev
