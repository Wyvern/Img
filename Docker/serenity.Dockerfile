FROM ubuntu:24.04

ENV DEBIAN_FRONTEND=noninteractive
ENV MAKEJOBS=4

RUN apt-get update && apt-get install -y --no-install-recommends \
    build-essential \
    gcc-14 \
    g++-14 \
    cmake \
    ninja-build \
    git \
    curl \
    ca-certificates \
    libmpfr-dev \
    libmpc-dev \
    libgmp-dev \
    e2fsprogs \
    qemu-system-gui \
    qemu-system-x86 \
    qemu-utils \
    ccache \
    rsync \
    unzip \
    texinfo \
    libssl-dev \
    zlib1g-dev \
    python3 \
    pkg-config \
    && rm -rf /var/lib/apt/lists/*

ENV CC=gcc-14
ENV CXX=g++-14

WORKDIR /src

RUN git clone --depth 1 https://github.com/SerenityOS/serenity.git

WORKDIR /src/serenity

RUN cd Toolchain && ARCH=x86_64 ./BuildGNU.sh

RUN cd Toolchain && ARCH=aarch64 ./BuildGNU.sh

CMD ["/bin/bash"]
