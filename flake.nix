{
  description = "CodeCrafters BitTorrent rust flake";

  inputs = {
    nixpkgs.url = "nixpkgs/release-24.05";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    flake-utils.follows = "rust-overlay/flake-utils";
  };

  outputs = {
    nixpkgs,
    rust-overlay,
    flake-utils,
    ...
  } @ inputs:
    flake-utils.lib.eachDefaultSystem (
      system: let
        overlays = [(import rust-overlay)];
        pkgs = import nixpkgs {
          inherit system overlays;
        };
      in {
        devShells.default = pkgs.mkShell {
          buildInputs = with pkgs; [
            (rust-bin.stable."1.77.2".default.override {
              extensions = ["rust-analyzer" "rust-src"];
            })
            bacon

            pkg-config
            openssl
          ];
        };
      }
    );
}
