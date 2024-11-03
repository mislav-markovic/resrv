alias b := build
alias r := serve


# build project
build:
  cargo build

# start the server that will serve assets
serve: build
  cargo run --bin server
