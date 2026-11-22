FROM rust:latest AS rust-builder
WORKDIR /src
COPY rust/ rust/
RUN cargo build --release --manifest-path rust/Cargo.toml

FROM golang:1.23 AS go-builder
WORKDIR /src
COPY go/ go/
COPY rust/include/ rust/include/
COPY --from=rust-builder /src/rust/target/release/libapifuzz_core.so /src/rust/target/release/
RUN cd go && CGO_ENABLED=1 go build -o /out/apifuzz-runner ./cmd/runner

FROM python:3.12-slim
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates curl \
    && rm -rf /var/lib/apt/lists/*
ENV LD_LIBRARY_PATH=/usr/local/lib PYTHONPATH=/opt/apifuzz/python
COPY --from=rust-builder /src/rust/target/release/libapifuzz_core.so /usr/local/lib/
COPY --from=go-builder /out/apifuzz-runner /usr/local/bin/apifuzz-runner
COPY python/ /opt/apifuzz/python/
COPY examples/ /opt/apifuzz/examples/
WORKDIR /opt/apifuzz
ENTRYPOINT ["apifuzz-runner"]
CMD ["-spec", "examples/openapi.json", "-base-url", "http://127.0.0.1:9101", "-out", "results.jsonl"]
