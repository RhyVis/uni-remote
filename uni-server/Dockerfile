FROM alpine:latest
RUN apk --no-cache add ca-certificates
WORKDIR /app
COPY target/x86_64-unknown-linux-musl/release/server /app/
ENTRYPOINT ["/app/uni-server"]
