ARG rust_ver

FROM docker.io/library/rust:${rust_ver}

LABEL rust_ver="${rust_ver}"

ARG check_container_dependencies_updated="true"

ARG binaryen_ver="version_117"

ENV CHECK_DEPENDENCIES_UPDATED="true"

VOLUME ["/artifacts"]

VOLUME ["/code"]

VOLUME ["/platform"]

RUN ["apt-get", "update"]

RUN ["apt-get", "install", "-y", "coreutils", "sed", "tar", "wget"]

RUN "rustup" "default" | "sed" "s/ (default)[[:space:]]\{0,\}//" \
  > "/rust-version"

RUN "echo" "${binaryen_ver}" | "tr" "-d" "\n" > "/binaryen-version"

RUN ["rustup", "target", "add", "wasm32-unknown-unknown"]

ADD "./tools/" "/tools/"

WORKDIR "/tools/"

RUN "[" "${check_container_dependencies_updated}" "=" "false" "]" || \
  "cargo" "update" "--locked"

RUN "cargo" "+$(cat "/rust-version")" "install" "--path" "/tools/cargo-each/"

WORKDIR "/"

RUN ["rm", "-rf", "/tools/"]

RUN ["mkdir", "/binaryen/"]

WORKDIR "/binaryen/"

RUN "wget" "-O" "binaryen.tar.gz" "https://github.com/WebAssembly/binaryen/\
releases/download/${binaryen_ver}/binaryen-${binaryen_ver}-x86_64-linux.tar.gz"

RUN ["tar", "-xf", "binaryen.tar.gz"]

RUN "mv" "-t" "/usr/bin/" "./binaryen-${binaryen_ver}/bin/wasm-opt"

WORKDIR "/"

RUN ["rm", "-rf", "/binaryen/"]

RUN ["mkdir", "/build/"]

ADD "./scripts/build-and-optimize.sh" "/build/build.sh"

RUN ["chmod", "-R", "a+rx-w", "/build/"]

RUN ["mkdir", "/target/"]

RUN ["mkdir", "/temp-artifacts/"]

CMD ["sh", "/build/build.sh"]
