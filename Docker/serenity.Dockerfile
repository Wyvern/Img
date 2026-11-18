# Build stage
FROM ubuntu:24.04 AS builder

ENV DEBIAN_FRONTEND=noninteractive
ENV MAKEJOBS=4
ENV CC=gcc-14
ENV CXX=g++-14

RUN apt-get update && apt-get install -y --no-install-recommends \
    gcc-14 \
    g++-14 \
    cmake \
    git \
    curl \
    ca-certificates \
    patch \
    xz-utils \
    libgmp-dev \
    libmpfr-dev \
    libmpc-dev \
    zlib1g-dev \
    && rm -rf /var/lib/apt/lists/*

RUN useradd -m -u 1000 builder

USER builder
WORKDIR /src

RUN git clone --depth 1 https://github.com/SerenityOS/serenity.git

WORKDIR /src/serenity

RUN cd Toolchain && ARCH=x86_64 ./BuildGNU.sh

RUN cd Toolchain && ARCH=aarch64 ./BuildGNU.sh


# Runtime/artifact stage
FROM ubuntu:24.04

RUN apt-get update && apt-get install -y --no-install-recommends \
    libgmp10 \
    libmpfr6 \
    libmpc3 \
    zlib1g \
    libstdc++6 \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /src/serenity/Toolchain/Local/ /src/serenity/Toolchain/Local/

COPY --from=builder /src/serenity/Build/x86_64/Root/ /src/serenity/Build/x86_64/Root/

COPY --from=builder /src/serenity/Build/aarch64/Root/ /src/serenity/Build/aarch64/Root/

ENV SERENITY_TOOLCHAIN=/src/serenity/Toolchain/Local

CMD ["/bin/bash"]
