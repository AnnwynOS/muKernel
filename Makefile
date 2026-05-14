.PHONY: build build-release run run-bios run-release debug check clippy install-deps help

help:
	cargo xtask help

build:
	cargo xtask build

build-release:
	cargo xtask build-release

run:
	cargo xtask run

run-bios:
	cargo xtask run-bios

run-release:
	cargo xtask run-release

debug:
	cargo xtask debug

check:
	cargo xtask check

clippy:
	cargo xtask clippy

install-deps:
	cargo xtask install-deps