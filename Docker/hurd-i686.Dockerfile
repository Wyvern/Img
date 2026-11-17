FROM ubuntu:24.04

ENV DEBIAN_FRONTEND=noninteractive
ENV PATH="/cross-tools-i686/bin:${PATH}"

RUN apt-get update && apt-get install -y --no-install-recommends \
    git sudo ca-certificates curl wget \
    build-essential perl \
    texinfo autopoint pkg-config libtool \
    autotools-dev automake autoconf gettext \
    libgmp-dev libmpfr-dev libmpc-dev zlib1g-dev \
    flex bison gawk m4 patch xz-utils bzip2 \
    file python3 \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /src/cross-hurd

RUN git clone --depth=1 https://github.com/flavioc/cross-hurd.git . \
    && sed -i 's|^ZLIB_VERSION=1\.3\.1$|ZLIB_VERSION=1.3.2|' package-versions.sh \
    && sed -i 's|^ZLIB_URL=.*|ZLIB_URL=https:\/\/zlib.net\/fossils\/"$ZLIB_PKG"|' download-funcs.sh \
    && sed -i 's|^MAKE_URL=\$GNU_REPO/"\$MAKE_PKG"|MAKE_URL=$GNU_REPO/make/"$MAKE_PKG"|' download-funcs.sh \
    && sed -i '/apply_patch .*submitted-AF_LINK\.diff 1/c\    sed -i -e "s|^[[:space:]]*#define[[:space:]]*PF_LINK[[:space:]]*18.*|/* #define PF_LINK 18 Link layer interface. */|" -e "s|^[[:space:]]*#define[[:space:]]*AF_LINK[[:space:]]*PF_LINK|/* #define AF_LINK PF_LINK */|" sysdeps/mach/hurd/bits/socket.h &&' download-funcs.sh \
    && sed -i 's|^DMIDECODE_URL=http://|DMIDECODE_URL=https://|' download-funcs.sh \
    && sed -i 's|download\.savannah\.gnu\.org/releases/dmidecode/|download-mirror.savannah.gnu.org/releases/dmidecode/|' download-funcs.sh \
    && sed -i '/--disable-libgomp \\/a\      --disable-libsanitizer \\' bootstrap-funcs.sh \
    && export CFLAGS="-O3 -pipe -g0" \
    && export CXXFLAGS="-O3 -pipe -g0" \
    && export LDFLAGS="-Wl,-O3" \
    && CPU=i686 bash -x download.sh \
    && DISABLE_ADA=1 CPU=i686 bash -x bootstrap.sh \
    && find /cross-tools-i686 -type d \
          \( -name __pycache__ -o -name man -o -name doc -o -name docs \) \
          -prune -exec rm -rf {} + \
    && rm -rf /src/cross-hurd/src \
              /src/cross-hurd/build-i686 \
              /root/.cache \
              /tmp/* \
              /var/tmp/* \
    && test -x /cross-tools-i686/bin/i686-gnu-gcc \
    && /cross-tools-i686/bin/i686-gnu-gcc -dumpmachine \
    && apt-get clean \
    && apt-get purge -y --auto-remove

ENTRYPOINT ["/cross-tools-i686/bin/i686-gnu-gcc"]
