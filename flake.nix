{
  description = "A basic flake with a shell";
  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
  inputs.flake-utils.url = "github:numtide/flake-utils";
  inputs.rust-overlay.url = "github:oxalica/rust-overlay";
  inputs.naersk.url = "github:nix-community/naersk";
  inputs.nvim-treesitter = {
    url = "github:nvim-treesitter/nvim-treesitter";
    flake = false;
  };

  outputs = {
    self,
    nixpkgs,
    flake-utils,
    rust-overlay,
    naersk,
    nvim-treesitter,
  }:
    flake-utils.lib.eachDefaultSystem (system: let
      pkgs = import nixpkgs {
        inherit system;

        overlays = [(import rust-overlay)];
      };
      rust = pkgs.rust-bin.stable.latest.default.override {
        extensions = ["llvm-tools-preview" "rust-src"];
      };

      naersk' = pkgs.callPackage naersk {
        cargo = rust;
        rustc = rust;
      };
    in {
      devShell = pkgs.mkShell {
        nativeBuildInputs = [
          pkgs.bashInteractive
          pkgs.cargo-watch
          rust
        ];
        buildInputs = [];

        shellHook = ''
          export NVIM_TREESITTER=${nvim-treesitter}
        '';
      };

      defaultPackage = naersk'.buildPackage {src = ./.;};
    });
}
