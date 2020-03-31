# yeeroot

[![standard-readme compliant](https://img.shields.io/badge/readme%20style-standard-brightgreen.svg?style=flat-square)](https://github.com/RichardLitt/standard-readme)

> Official implementation of the YeeCo Root Chain (Layer 1)

YeeCo is a permissionless, secure, high performance and scalable public blockchain platform powered by full sharding technology on PoW consensus.

  
📣 YeeCo Testnet launched! (2020-03-31)
 - [View release notes](https://github.com/yeeco/wiki/blob/master/docs/release-notes/testnet-release-notes.md) 
 - [View blockchain explorer](https://testnet.yeescan.org/)

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
tyee1jfakj2rvqym79lmxcmjkraep6tn296deyspd9mkh467u4xgqt3cqkv6lyl    0            0xa8666e483fd6c26dbb6deeec5afae765561ecc94df432f02920fc5d9cd4ae206ead577e5bc11215d4735cee89218e22f2d950a2a4667745ea1b5ea8b26bba5d6
tyee15zphhp8wmtupkf3j8uz5y6eeamkmknfgs6rj0hsyt6m8ntpvndvsmz3h3w    1            0x40e17c894e03256ea7cb671d79bcc88276c3fd6e6a05e9c0a9546c228d1f4955d8f18e85255020c97764251977b77f3b9e18f4d6de7b62522ab29a49cede669f
tyee14t6jxhs885azsd9v4t75cre9t4crv6a89q2vg8472u3tvwm3f94qgr9w77    2            0x708084bc9da56d9d1b201f50830269887ff2ef74e619c6af6ba7cf506068326f7cc9c4d646c531e83507928114ff9ef66350c62dfda3a7c5d2f0d9e0c37e7750
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
1. **[Done]** PoC-6: Cross chain (interoperate with branch chain) (2019-12)

    [Release notes](https://github.com/yeeco/wiki/blob/master/docs/release-notes/poc6-release-notes.md)
1. **[Done]** PoC-7: Smart contract (on branch chain) (2020-02-14)

    [Release notes](https://github.com/yeeco/wiki/blob/master/docs/release-notes/poc7-release-notes.md)

1. **[Done]** Testnet (2020-03)

    [Release notes](https://github.com/yeeco/wiki/blob/master/docs/release-notes/testnet-release-notes.md)
    
1. **[In Progress]** Mainnet (2020-06)

## Contributing

Feel free to dive in! [Open an issue](./issues/new).

### Contributors


## License

[GPL](LICENSE)