{
  pkgs ? import ./nix/nixpkgs-compat.nix,
  packages ? pkgs.callPackage ./nix/packages { },
}:
pkgs.mkShell {
  packages =
    with builtins;
    attrValues (
      removeAttrs packages [
        "rust-nightly"
      ]
    );
}
