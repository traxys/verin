{
  description = "A basic flake with a shell";
  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
  inputs.flake-utils.url = "github:numtide/flake-utils";
  inputs.rust-overlay.url = "github:oxalica/rust-overlay";
  inputs.rust-overlay.inputs.nixpkgs.follows = "nixpkgs";
  inputs.crane.url = "github:ipetkov/crane";
  inputs.crane.inputs.nixpkgs.follows = "nixpkgs";
  inputs.nvim-treesitter = {
    url = "github:nvim-treesitter/nvim-treesitter";
    flake = false;
  };

  outputs = {
    self,
    nixpkgs,
    flake-utils,
    rust-overlay,
    crane,
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
    in {
      devShell = pkgs.mkShell {
        nativeBuildInputs = [
          pkgs.bashInteractive
          rust
        ];
        buildInputs = [];

        shellHook = ''
          export NVIM_TREESITTER=${nvim-treesitter}
        '';
      };

      packages = let
        craneLib = (crane.mkLib pkgs).overrideToolchain rust;
      in {
        default = craneLib.buildPackage {
          pname = "verin";
          version =
            if (self ? shortRev)
            then self.shortRev
            else if (self ? dirtyShortRev)
            then self.dirtyShortRev
            else "unknown";

          src = craneLib.cleanCargoSource (craneLib.path ./.);
          NVIM_TREESITTER = "${nvim-treesitter}";
        };
      };
    });
}
