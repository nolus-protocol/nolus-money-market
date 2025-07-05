{
  pkgs ? import ../nixpkgs-compat.nix,
}:
with pkgs;
let
  rustPlatform =
    rust:
    makeRustPlatform {
      cargo = rust;

      rustc = rust;
    };

  rust-nightly = rust-bin.nightly."2025-07-01".minimal;

  rust-stable = rust-bin.stable."1.87.0".minimal.override {
    extensions = [
      "clippy"
      "rustfmt"
    ];
  };

  stableRustPlatform = rustPlatform rust-stable;

  mkEnvScript = callPackage (
    {
      writeShellScriptBin,
    }:
    script: var:
    writeShellScriptBin script ''
      set -eu

      case "''${#}" in
        ("0")
          "echo" "''${${var}}"
          ;;
        (*)
          ${var}="''${1}"
          export ${var}
          shift

          "''${@}"
      esac
    ''
  ) { };
in
rec {
  inherit
    coreutils
    gzip
    rust-nightly
    rust-stable
    ;

  cargo-each = callPackage ./cargo-each.nix {
    inherit stableRustPlatform;
  };

  check-formatting = callPackage ./check-formatting.nix {
    inherit rust-stable;
  };

  check-lockfiles = callPackage ./check-lockfiles.nix {
    inherit rust-stable;
  };

  check-package-unused-deps = callPackage ./check-package-unused-deps.nix {
    inherit
      check-unused-deps
      for-combinations
      parse-package-name
      profile
      set-ci-env
      ;
  };

  check-unused-deps = callPackage ./check-unused-deps.nix {
    inherit rust-nightly;
  };

  ci-check-formatting = callPackage ./ci-check-formatting.nix {
    inherit check-formatting for-workspaces;
  };

  ci-check-lockfiles = callPackage ./ci-check-lockfiles.nix {
    inherit check-lockfiles for-workspaces;
  };

  ci-check-unused-deps = callPackage ./ci-check-unused-deps.nix {
    inherit
      check-unused-deps
      for-combinations
      for-workspaces
      set-ci-env
      ;
  };

  ci-lint = callPackage ./ci-lint.nix {
    inherit
      for-combinations
      for-workspaces
      lint
      profile
      set-ci-env
      ;
  };

  ci-profiles-json = callPackage ./ci-profiles-json.nix {
    inherit
      for-profiles
      profile
      set-ci-env
      ;
  };

  ci-run-tests = callPackage ./ci-run-tests.nix {
    inherit
      for-combinations
      for-workspaces
      profile
      run-tests
      set-ci-env
      ;
  };

  create-tar-archive = callPackage ./create-tar-archive.nix { };

  dex = mkEnvScript "dex" "USED_DEX";

  for-combinations = callPackage ./for-combinations.nix {
    inherit
      cargo-each
      dex
      profile-mode
      rust-stable
      ;
  };

  for-profiles = callPackage ./for-profiles.nix {
    inherit profile profile-mode;
  };

  for-workspaces = callPackage ./for-workspaces.nix { };

  generate-release-notes = callPackage ./generate-release-notes.nix { };

  lint = callPackage ./lint.nix {
    inherit rust-stable;
  };

  lint-package = callPackage ./lint-package.nix {
    inherit
      for-combinations
      for-profiles
      lint
      parse-package-name
      ;
  };

  parse-package-name = callPackage ./parse-package-name.nix { };

  profile = mkEnvScript "profile" "USED_PROFILE";

  profile-mode = mkEnvScript "profile-mode" "USED_PROFILE_MODE";

  run-package-tests = callPackage ./run-package-tests.nix {
    inherit
      for-combinations
      for-profiles
      parse-package-name
      run-tests
      set-ci-env
      ;
  };

  run-tests = callPackage ./run-tests.nix {
    inherit profile rust-stable;
  };

  set-ci-env = callPackage ./set-ci-env.nix {
    inherit profile-mode;
  };
}
