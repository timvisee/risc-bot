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
		build-essential \
		ca-certificates \
		cmake \
		cowsay \
		curl \
		dnsutils \
		fortune \
		git \
		golang \
		iputils-ping \
		iputils-tracepath \
		libssl-dev \
		nodejs \
		openssl \
		php \
		pkg-config \
		python \
		python3 \
		ruby-full \
		sudo \
		toilet \
		translate-shell \
		vim \
		wget 

# Install Rust, update the cargo package index
RUN curl https://sh.rustup.rs -sSf | sh -s -- -y --default-toolchain stable
RUN cargo install ffsend

# Clean apt but keep the package index alive
RUN apt clean
RUN apt update -yq

CMD ["/bin/bash"]
