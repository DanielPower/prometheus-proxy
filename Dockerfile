FROM rust:1.86 as builder
WORKDIR /app
COPY . .
RUN cargo build --release

FROM alpine:3.21
RUN apk --no-cache add ca-certificates
COPY --from=builder /app/target/release/prometheus-proxy /app/prometheus-proxy
EXPOSE 8080

CMD ["/app/prometheus-proxy"]
