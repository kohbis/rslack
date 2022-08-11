FROM ekidd/rust-musl-builder:stable as builder
# defined in ekidd/rust-musl-builder
# WORKDIR /home/rust/src
ADD . .
RUN cargo build --release --target x86_64-unknown-linux-musl

FROM scratch as runner
COPY --from=builder /etc/ssl/certs/ca-certificates.crt /etc/ssl/certs/ca-certificates.crt
COPY --from=builder /home/rust/src/target/x86_64-unknown-linux-musl/release/rslack /usr/local/bin/rslack
COPY .token .token
ENTRYPOINT [ "rslack" ]
