import <nixpkgs> {
  overlays = [
    (import <rust-overlay>)
  ];
}
