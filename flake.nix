{
  description = "Flake for working with the Nolus money market project.";

  inputs = {
    flake-utils.url = "github:numtide/flake-utils";

    nixpkgs.url = "github:nixos/nixpkgs?ref=nixos-unstable";

    rust-overlay = {
      inputs.nixpkgs.follows = "nixpkgs";

      url = "github:oxalica/rust-overlay";
    };
  };

  outputs =
    {
      flake-utils,
      nixpkgs,
      rust-overlay,
      ...
    }:
    flake-utils.lib.eachDefaultSystem (
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
      let
        packages = import ./nix/packages {
          inherit pkgs;
        };
      in
      {
        packages =
          assert (!(packages ? "default"));
          packages
          // {
            default = symlinkJoin {
              name = "all";

              paths = builtins.attrValues packages;
            };
          };

        devShells = {
          default = callPackage ./shell.nix {
            inherit packages;
          };
        };
      }
    );
}
