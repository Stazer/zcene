# zcene
zcene is a research actor model platform for hosted and baremetal environments.

# Project Hierarchy

# Dependencies
- Rust Nightly
- Cargo Make
- NASM
- LLD
- QEMU

## Install Dependencies 

### FreeBSD

#### Building

``` shell
pkg install -y nasm

rustup install nightly
rustup target add x86_64-unknown-none
rustup component add rust-src
rustup component add llvm-tools-preview

cargo install cargo-binutils
cargo install cargo-make
```

### MacOS (x86)

#### Building

``` shell
brew install rustup nasm lld

rustup install nightly
rustup target add x86_64-unknown-none
rustup component add rust-src llvm-tools-preview

cargo install cargo-binutils cargo-make
```

### Debian

#### Building

``` shell
apt install nasm rustup lld

rustup install nightly
rustup target add x86_64-unknown-none
rustup component add rust-src llvm-tools-preview

cargo install cargo-binutils cargo-make
```
