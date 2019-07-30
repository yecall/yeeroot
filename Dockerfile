FROM alpine:edge AS builder

RUN apk add \
    build-base \
    cmake \
    linux-headers \
    openssl-dev \
    clang-dev \
    cargo

ARG PROFILE=release
WORKDIR /yeeroot
COPY . /yeeroot

RUN mkdir -p /yeeroot/runtime/wasm/target/wasm32-unknown-unknown/release && \
    ln -s ../../../../../prebuilt/yee_runtime/poc_testnet.wasm \
        /yeeroot/runtime/wasm/target/wasm32-unknown-unknown/release/yee_runtime_wasm.compact.wasm && \
    cargo build --$PROFILE

# Pull yee from builder to deploy container
FROM alpine:latest

ARG PROFILE=release
COPY --from=builder /yeeroot/target/$PROFILE/yee /usr/local/bin

RUN apk add --no-cache \
    ca-certificates \
    libstdc++ \
    openssl

RUN mkdir -p /root/.local/share/YeeRoot && \
    ln -s /root/.local/share/YeeRoot /data

EXPOSE 30333 9933 9944
VOLUME ["/data"]

CMD ["/usr/local/bin/yee"]
