FROM rust:1.86 as builder
WORKDIR /usr/src/prometheus-proxy
COPY . .
RUN cargo build --release

FROM alpine:3.19
RUN apk --no-cache add ca-certificates
COPY --from=builder /usr/src/prometheus-proxy/target/release/prometheus-proxy /usr/local/bin/prometheus-proxy
EXPOSE 8080

CMD ["prometheus-proxy"]
