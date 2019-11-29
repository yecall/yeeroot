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
1. Openssl
1. Rust nightly
    ```sh
    rustup toolchain add nightly
    ```
1. rust nightly wasm
    ```sh
    rustup target add wasm32-unknown-unknown
    rustup target add wasm32-unknown-unknown --toolchain nightly
    ```
1. wasm-gc
    ```sh
    cargo install wasm-gc
    ```
1. Rust components: clippy rls docs src rustfmt
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

### Start

1. Start bootnodes router.
    ```sh
    $ ./yee bootnodes-router --dev-params
    ```
    Bootnodes router provides the bootnodes for each shard.
    
    You can try getting the bootnodes by the following RPC: 
    ```sh
    $ curl -X POST --data '{"jsonrpc":"2.0","method":"bootnodes","params":[],"id":1}' localhost:50001 -H 'Content-Type: application/json'
    ```

1. Start the nodes of the 4 shards
    ```sh
    $ ./yee --dev --dev-params --shard-num=0 --base-path=/tmp/yee/shard_0
    $ ./yee --dev --dev-params --shard-num=1 --base-path=/tmp/yee/shard_1
    $ ./yee --dev --dev-params --shard-num=2 --base-path=/tmp/yee/shard_2
    $ ./yee --dev --dev-params --shard-num=3 --base-path=/tmp/yee/shard_3
    ```
    Since we start the node without `--mine`, it will not mine new blocks.


1. Start switch
    ```sh
    $ ./yee switch --dev-params --mine
    ```
    Switch provides proxy rpc of all the 4 shards.
    You can get the balance of a certain address of any shard by the following RPC: 
    ```sh
    $ curl -X POST --data '{"jsonrpc":"2.0","method":"state_getBalance","params":["tyee15c2cc2uj34w5jkfzxe4dndpnngprxe4nytaj9axmzf63ur4f8awq806lv6"],"id":1}' localhost:10033 -H 'Content-Type: application/json'
    ```
    
    Switch can also work as a multi-miner. Since we start the switch with `--mine`, it will mine on the 4 shards.

### Accounts

Test accounts: 
    
```
Address                                                            Shard num    Private key
tyee15c2cc2uj34w5jkfzxe4dndpnngprxe4nytaj9axmzf63ur4f8awq806lv6    0            0xf8eb0d437140e458ec6103965a4442f6b00e37943142017e9856f3310023ab530a0cc96e386686f95d2da0c7fa423ab7b84d5076b3ba6e7756e21aaafe9d3696
tyee10n605lxn7k7rfm4t9nx3jd6lu790m30hs37j7dvm6jeun2kkfg7sf6fp9j    1            0xd0542cb78c304aa7ea075c93772d2a8283b75ea218eb9d6dd96ee181fc9da26caa746ccc1625cbd7451c25860c268792f57f108d536034173a42353ced9cf1e1
tyee16pa6aa7qnf6w5ztqdvla6kvmeg78pkmpd76d98evl88ppmarcctqdz5nu3    2            0xa8f84e392246b1a4317b1deb904a8272c0428d3d324e1889be8f00b0500a1e63845dbc96f4726783d94d7edcdeb8878ce4dcac793c41e815942c664687599c19
tyee12n2pjuwa5hukpnxjt49q5fal7m5h2ddtxxlju0yepzxty2e2fads5g57yd    3            0xa079ef650520662d08f270c4bc088f0c61abd0224f58243f6d1e6827c3ab234a7a1a0a3b89bbb02f2b10e357fd2a5ddb5050bc528c875a6990874f9dc6496772
```
    

## Roadmap
1. **[Done]** PoC-1: Tetris consensus demo (2019-02)

     [tetris_demo](https://github.com/yeeco/tetris_demo)
2. **[Done]** PoC-2: Transfer feature based on Tetris (2019-05)

     [gyee](https://github.com/yeeco/gyee)
1. **[Done]** PoC-3: PoW consensus, static sharding (2019-07)
    
    [Release notes](https://github.com/yeeco/wiki/blob/master/docs/release-notes/poc3-release-notes.md)
1. **[Done]** PoC-4: Multi-mining, cross-shard transactions (2019-09)

    [Release notes](https://github.com/yeeco/wiki/blob/master/docs/release-notes/poc4-release-notes.md)
1. **[Done]** PoC-5: Dynamic sharding (2019-11)

    [Release notes](https://github.com/yeeco/wiki/blob/master/docs/release-notes/poc5-release-notes.md)
1. **[In progress]** PoC-6: Cross chain (interoperate with branch chain) (2019-12)
1. PoC-7: Smart contract (on branch chain) (2020-01)
1. Testnet (2020-03)
1. Mainnet (2020-06)

## Contributing

Feel free to dive in! [Open an issue](./issues/new).

### Contributors


## License

[GPL](LICENSE)