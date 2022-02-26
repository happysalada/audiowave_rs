{
  description = "Just a shell for now";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixpkgs-unstable";
    rust-overlay.url = "github:oxalica/rust-overlay";
  };

  outputs = { self, nixpkgs, rust-overlay, flake-utils, ... }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs {
          inherit system overlays;
        };
        rust-binaries = pkgs.rust-bin.beta.latest.default.override {
          targets = [ "wasm32-unknown-unknown" ];
        };
      in
      {
        devShell = pkgs.mkShell {
          buildInputs = with pkgs; [
              rust-binaries
              worker-build
              wrangler
          ];

          shellHook = ''
          '';
        };
      }
    );
}
