{
  stableRustPlatform,
}:
stableRustPlatform.buildRustPackage (final: rec {
  pname = "cargo-each";
  version = "0.0.0";

  src = ../../tools;

  cargoLock.lockFile = src + "/Cargo.lock";
})
