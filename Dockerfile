FROM rust:1.40 as build
WORKDIR /usr/src/racemus

RUN rustup toolchain install nightly
COPY . .
RUN cargo build --release

FROM debian:buster-slim
WORKDIR /root
RUN apt-get update \
    # racemus deps
    && apt-get install -y \
        # required by surf
        libcurl4 \
        # required by docker-start.sh
        openssl \
    # Clean up
    && apt-get autoremove -y \
    && apt-get clean -y \
    && rm -rf /var/lib/apt/lists/*
COPY --from=build /usr/src/racemus/target/release/racemus /usr/local/bin/racemus
COPY --from=build /usr/src/racemus/server.toml ./server.toml
COPY --from=build /usr/src/racemus/docker-start.sh ./docker-start.sh
ENV RACEMUS_LOG racemus=trace
EXPOSE 25565/tcp
ENTRYPOINT [ "sh", "/root/docker-start.sh" ]
