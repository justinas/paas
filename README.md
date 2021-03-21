# Paas (processes as a service)

<!-- vim-markdown-toc GFM -->

* [Building](#building)
* [Running](#running)
    * [Paasd (server)](#paasd-server)
    * [Paasc (client) example session](#paasc-client-example-session)
* [Contributing](#contributing)

<!-- vim-markdown-toc -->

## Building

Requirements: 
* Rust 1.45.2 or higher
* Protoc (installed in a standard location, see [prost-build docs](https://docs.rs/prost-build/0.7.0/prost_build/#sourcing-protoc) for more info)

Nix users can use the provided [shell.nix](/shell.nix).

To compile in release profile, run:

```console
$ cargo build --release
```

## Running 

### Paasd (server)

```console
$ RUST_LOG=info ./target/release/paasd
 INFO  paasd > starting on 127.0.0.1:8443
```

### Paasc (client) example session
```console
$ ./target/release/paasc exec bash -c 'while true; do echo $RANDOM; sleep 1; done'
50a1d407-c48c-4323-854d-a954aca0b075
$ ./target/release/paasc status 50a1d407-c48c-4323-854d-a954aca0b075
Status: running
$ ./target/release/paasc logs 50a1d407-c48c-4323-854d-a954aca0b075 
26313
5952
2737
^C
$ ./target/release/paasc stop 50a1d407-c48c-4323-854d-a954aca0b075
$ ./target/release/paasc status 50a1d407-c48c-4323-854d-a954aca0b075
Status: exited (signal 15)
```

## Contributing

Code style is enforced by [rustfmt](https://github.com/rust-lang/rustfmt).
[Clippy](https://github.com/rust-lang/rust-clippy) is used as a linter.
