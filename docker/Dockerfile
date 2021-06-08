FROM ubuntu

LABEL maintainer="Tim Visee <3a4fb3964f@sinenomine.email>"

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
		gnupg \
		libgpgme11 \
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
RUN wget "https://github.com/timvisee/ffsend/releases/download/v0.2.72/ffsend-v0.2.72-linux-x64-static" -O /usr/bin/ffsend \
 && chmod a+x /usr/bin/ffsend
RUN wget "https://github.com/timvisee/prs/releases/download/v0.2.11/prs-gtk3-copy-v0.2.11-linux-x64-static" -O /usr/bin/prs \
 && chmod a+x /usr/bin/prs

# Clean apt but keep the package index alive
RUN apt clean
RUN apt update -yq

CMD ["/bin/bash"]
