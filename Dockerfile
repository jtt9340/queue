# Adapted from https://blog.sedrik.se/posts/my-docker-setup-for-rust/
FROM ekidd/rust-musl-builder:stable as builder
MAINTAINER Joey Territo (jtt9340@rit.edu)

WORKDIR /home/rust

# Avoid having to install/build all dependencies by copying
# the Cargo files and making a dummy src/main.rs
COPY Cargo.toml .
COPY Cargo.lock .
COPY src/lib.rs src
RUN mkdir src/bin src/example && \
touch src/queue.rs src/user.rs
RUN echo "fn main() {}" | tee src/bin/main.rs > src/bin/clear_file.rs
RUN cargo test --bin slack_main
RUN cargo build --release

# We need to touch our real main.rs file or else Docker will use the cached one.
COPY . .
RUN sudo touch src/bin/main.rs src/user.rs

RUN cargo test
RUN cargo build --release

# Size optimization
RUN strip target/x86_64-unknown-linux-musl/release/slack_main

# Start building the final image
FROM scratch
WORKDIR /home/rust
COPY --from=builder /home/rust/target/x86_64-unknown-linux-musl/release/slack_main .
ENTRYPOINT ["./slack_main"]
