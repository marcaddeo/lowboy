chooser := "grep -v choose | fzf --tmux"

# Display this list of available commands
@list:
    just --justfile "{{ source_file() }}" --list

alias c := choose
# Open an interactive chooser of available commands
[no-exit-message]
@choose:
    just --justfile "{{ source_file() }}" --chooser "{{ chooser }}" --choose 2>/dev/null

alias e := edit
# Edit the justfile
@edit:
    $EDITOR "{{ justfile() }}"

_ensure_node_modules:
    test -d node_modules || npm install

build-css: _ensure_node_modules
    npx tailwindcss -i css/styles.css -o css/styles.dist.css

build-server:
    cargo build

alias b := build
# Build project
build: build-css build-server

alias w := build
[doc("Watch & build project")]
watch:
    #!/usr/bin/env bash
    cargo watch -x run &
    server_pid="$!"
    trap "kill $server_pid; exit" SIGINT
    npx tailwindcss -i css/styles.css -o css/styles.dist.css --watch
