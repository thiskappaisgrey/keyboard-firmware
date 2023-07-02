{
  inputs = {
    nixpkgs.url = "nixpkgs/nixos-unstable";
    utils.url = "github:numtide/flake-utils";
    rust-overlay.url = "github:oxalica/rust-overlay";
  };
  description = "A very basic flake";

  outputs = { self, nixpkgs, utils, rust-overlay }: utils.lib.eachDefaultSystem(system:
    let overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs { inherit system overlays; };
    in rec
      {
        devShell = pkgs.mkShell {
          buildInputs = with pkgs;[
            (rust-bin.nightly.latest.default.override {
              #   extensions = ["llvm-tools-preview"];
                targets = [ "thumbv6m-none-eabi" ];

            })
            probe-run
            flip-link
            elf2uf2-rs
 
          ];
        };

      });
}
