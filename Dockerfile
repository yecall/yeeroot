FROM rust:1.46 AS builder

RUN set -eux; \
    apt-get update; \
    apt-get install -y --no-install-recommends \
        cmake pkg-config libssl-dev git clang libclang-dev

WORKDIR /yeeroot
COPY . /yeeroot

RUN mkdir -p /yeeroot/runtime/wasm/target/wasm32-unknown-unknown/release && \
    ln -s ../../../../../prebuilt/yee_runtime/mainnet.wasm \
        /yeeroot/runtime/wasm/target/wasm32-unknown-unknown/release/yee_runtime_wasm.compact.wasm && \
    cargo build --release

# Pull yee from builder to deploy container
FROM debian:stretch

COPY --from=builder /yeeroot/target/release/yee /usr/local/bin

RUN set -eux; \
    apt-get update; \
    apt-get install -y --no-install-recommends \
        ca-certificates openssl \
    ; \
    rm -rf /var/lib/apt/lists/*;

RUN mkdir -p /root/.local/share/YeeRoot && \
    ln -s /root/.local/share/YeeRoot /data

EXPOSE 30333 30334 9933 9944
VOLUME ["/data"]

CMD ["/usr/local/bin/yee"]
