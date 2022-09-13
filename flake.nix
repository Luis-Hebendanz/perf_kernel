# This file is pretty general, and you can adapt it in your project replacing
# only `name` and `description` below.

{
  description = "My awesome Rust project";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    nixos-codium = {
      url = "github:luis-hebendanz/nixos-codium";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    nci = {
      url = "github:yusdacra/nix-cargo-integration";
      inputs.nixpkgs.follows = "nixpkgs";
      inputs.rust-overlay.follows = "rust-overlay";
    };
  };

  outputs = { self, nixpkgs, nci, rust-overlay, flake-utils, nixos-codium, ... }:
    flake-utils.lib.eachSystem [ "x86_64-linux" ] (system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs {
          inherit system overlays;
        };
        myrust = pkgs.rust-bin.nightly."2021-10-26".default.override {
          extensions = [ "rustfmt" "llvm-tools-preview" "rust-src" ];
        };

        mycodium = import ./vscode { vscode = nixos-codium.packages.${system}.default; inherit pkgs; };
      in
      {
        devShells.default = pkgs.mkShell {
          buildInputs = with pkgs; [
            mycodium
            myrust
            evcxr # rust repl
            cargo-tarpaulin # for code coverage
            rust-analyzer
            zlib.out
            xorriso
            dhcp
            grub2
            qemu
            entr # bash file change detector
            #glibc.dev # Creates problems with tracy
            netcat-gnu
            git-extras
            python3
          ] ++ (with pkgs.python39Packages; [
            pyelftools
            intervaltree
          ]) ++ (with pkgs.llvmPackages_latest; [
            lld
            bintools
            llvm
          ]);

          LIBCLANG_PATH= pkgs.lib.makeLibraryPath [ pkgs.llvmPackages_latest.libclang.lib ];
        
          BINDGEN_EXTRA_CLANG_ARGS = 
          # Includes with normal include path
          (builtins.map (a: ''-I"${a}/include"'') [
            pkgs.glibc.dev 
          ])
          # Includes with special directory paths
          ++ [
            ''-I"${pkgs.llvmPackages_latest.libclang.lib}/lib/clang/${pkgs.llvmPackages_latest.libclang.version}/include"''
          ];

          HISTFILE=toString ./.history;
        };

      });
}
