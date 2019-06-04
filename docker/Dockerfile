FROM ubuntu

LABEL maintainer="Tim Visee <timvisee@gmail.com>"

ENV RUSTUP_HOME=/usr/local/rustup \
    CARGO_HOME=/usr/local/cargo \
    PATH=/usr/local/cargo/bin:/usr/games:$PATH \
    DEBIAN_FRONTEND=noninteractive

# Upgrade the system, install common utilities
RUN apt update -yq
RUN apt upgrade -yq
RUN apt install -yq \
		git \
		vim \
		iputils-ping \
		iputils-tracepath \
		wget \
		curl \
		cowsay \
		fortune \
		toilet \
		nodejs \
		php \
		python \
		python3 \
		build-essential \
		sudo \
		openssl \
		ca-certificates \
		cmake \
		pkg-config \
		libssl-dev

# Install Rust, update the cargo package index
RUN curl https://sh.rustup.rs -sSf | sh -s -- -y --default-toolchain stable
RUN cargo install ffsend

# Clean apt but keep the package index alive
RUN apt clean
RUN apt update -yq

CMD ["/bin/bash"]
