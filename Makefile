.PHONY: build install run test clean

BINARY_NAME=koralreef
PROJECT_NAME=kora-reclaim-rs
PREFIX ?= $(HOME)/.local
BINDIR = $(PREFIX)/bin

build:
	cargo build --release

install: build
	mkdir -p $(BINDIR)
	cp target/release/$(PROJECT_NAME) $(BINDIR)/$(BINARY_NAME)
	chmod +x $(BINDIR)/$(BINARY_NAME)
	@echo "Installed to $(BINDIR)/$(BINARY_NAME)"

run:
	cargo run -- --config config.toml.example

test:
	cargo test

clean:
	cargo clean
