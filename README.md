# yeeroot

[![standard-readme compliant](https://img.shields.io/badge/readme%20style-standard-brightgreen.svg?style=flat-square)](https://github.com/RichardLitt/standard-readme)

> Official implementation of the YeeCo Root Chain (Layer 1)

YeeCo is a permissionless, secure, high performance and scalable public blockchain platform powered by full sharding technology on PoW consensus.

## Table of Contents

- [Description](#description)
- [Install](#install)
- [Usage](#usage)
- [Roadmap](#roadmap)
- [Contributing](#contributing)
- [License](#license)

## Description

Yeeco takes advantage of the 4 key mechanisms as follows:

1. PoW (permissionless and secure)
2. Full sharding (high performance and scalable)
3. Multi-mining (resist 1% attack)
4. CRFG (conditional reward finality gadget, introduce absolute finality on PoW to support lock-free cross shard transactions)

![](https://raw.githubusercontent.com/yeeco/wiki/master/assets/images/yeeco-mechanisms.jpg)

## Install

### Requirements
1. Rust
    ```sh
    curl https://sh.rustup.rs -sSf | sh
    ```
2. Openssl
3. Rust nightly
    ```sh
    rustup toolchain add nightly
    ```
4. rust nightly wasm
    ```sh
    rustup target add wasm32-unknown-unknown
    rustup target add wasm32-unknown-unknown --toolchain nightly
    ```
5. wasm-gc
    ```sh
    cargo install wasm-gc
    ```
6. Rust components: clippy rls docs src rustfmt
    ```sh
    rustup component list # list all the components installed
    rustup component add <name> # install component
    ```

### Building
```sh
$ cd <project_base_dir>/runtime/wasm
$ sh build.sh
$ cd <project_base_dir>
$ cargo build
```

## Usage
devepment
```sh
$ ./yee --dev
```

## Roadmap
1. **[Done in [tetris_demo](https://github.com/yeeco/tetris_demo)]** PoC-1: Tetris cconsensus demo (2019-02)
2. **[Done in [gyee](https://github.com/yeeco/gyee)]** PoC-1: Tetris cconsensus demo (2019-05)
1. **[In progress]** PoC-3: PoW consensus, static sharding (2019-07)
1. PoC-4: Multi-mining, cross-shard transactions (2019-09)
1. PoC-5: Dynamic sharding (2019-11)
1. PoC-6: Cross chain (interoperate with branch chain) (2019-12)
1. PoC-7: Smart contract (on branch chain) (2020-01)
1. Testnet (2020-03)
1. Mainnet (2020-06)

## Contributing

Feel free to dive in! [Open an issue](./issues/new).

### Contributors


## License

[GPL](LICENSE)