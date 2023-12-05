ARG rust_ver

FROM docker.io/library/rust:${rust_ver}

ARG binaryen_ver="version_116"

RUN ["rustup", "target", "add", "wasm32-unknown-unknown"]

VOLUME ["/artifacts"]

VOLUME ["/code"]

WORKDIR "/"

ADD "./tools/cargo-features/" "/cargo-features/"

RUN ["cargo", "install", "--path", "/cargo-features/"]

RUN ["rm", "-rf", "/cargo-features/"]

RUN ["mkdir", "/binaryen/"]

WORKDIR "/binaryen/"

RUN ["apt-get", "update"]

RUN ["apt-get", "install", "-y", "coreutils", "sed", "tar", "wget"]

RUN "wget" "-O" "binaryen.tar.gz" "https://github.com/WebAssembly/binaryen/releases/download/${binaryen_ver}/binaryen-${binaryen_ver}-x86_64-linux.tar.gz"

RUN ["tar", "-xf", "binaryen.tar.gz"]

RUN "mv" "-t" "/usr/bin/" "./binaryen-${binaryen_ver}/bin/wasm-opt"

WORKDIR "/"

RUN ["rm", "-rf", "/binaryen/"]

RUN ["mkdir", "/build/"]

ADD "./scripts/build-and-optimize.sh" "/build/build.sh"

RUN ["chmod", "-R", "a+rx-w", "/build/"]

RUN ["mkdir", "/target/"]

RUN ["mkdir", "/temp-artifacts/"]

CMD ["bash", "/build/build.sh"]
