{
  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-25.05";

    rust-overlay = {
      inputs.nixpkgs.follows = "nixpkgs";

      url = "github:oxalica/rust-overlay";
    };
  };

  outputs =
    {
      self,
      nixpkgs,
      rust-overlay,
    }:
    let
      forEachSystem =
        with builtins;
        output:
        foldl'
          (
            accumulator: system:
            mapAttrs (
              name: value:
              (if accumulator ? ${name} then accumulator."${name}" else { })
              // {
                ${system} = value;
              }
            ) (output system)
          )
          { }
          [
            "x86_64-linux"
          ];
    in
    forEachSystem (
      system:
      let
        pkgs = import nixpkgs {
          inherit system;

          overlays = [
            rust-overlay.overlays.default
          ];
        };
      in
      with pkgs;
      {
        packages = rec {
          cargo-each =
            let
              rustPlatform =
                let
                  rust = rust-bin.stable.latest.minimal;
                in
                makeRustPlatform {
                  cargo = rust;

                  rustc = rust;
                };

              src = ./tools;
            in
            rustPlatform.buildRustPackage {
              name = "cargo-each";

              version = "0.0.0";

              src = src;

              cargoLock.lockFile = src + "/Cargo.lock";
            };

          lint = writeShellScriptBin "lint.sh" ''
            set -eu

            case "''${#}" in
              ("1") ;;
              (*)
                "echo" \
                  "This script takes only one argument, the workspace name!" \
                  >&2

                exit "1"
            esac

            cd "./''${1:?}"
            shift

            PATH="${
              lib.makeBinPath [
                cargo-each
                (rust-bin.stable."1.86.0".minimal.override {
                  extensions = [
                    "clippy"
                  ];
                })
                stdenv.cc
              ]
            }:''${PATH:?}" \
              SOFTWARE_RELEASE_ID="local" \
              PROTOCOL_NETWORK="local" \
              PROTOCOL_NAME="local" \
              PROTOCOL_RELEASE_ID="local" \
              "cargo" \
              "lint"
          '';

          regression = writeShellScriptBin "regression.sh" ''
            PROFILE="ci_dev_no_debug_assertions" \
              nix \
              shell \
              --ignore-environment \
              --keep-env-var "PROFILE" \
              "nixpkgs#nix" \
              --command \
              nix \
              run \
              ".#lint" \
              "platform/"
          '';
        };
      }
    );
}
