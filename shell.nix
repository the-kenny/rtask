{ nixpkgs ? <nixpkgs> }:
let
  pkgs = import nixpkgs {};
in rec {
  rustEnv = pkgs.stdenv.mkDerivation {
    name = "rust";
    version = "1.2.3.4";
    src = ./.;
    buildInputs = with pkgs; [ rustup pkgconfig sqlite openssl gdb ];

    RUST_LOG="rtask=info";
    RUST_BACKTRACE=0;

    RTASK_DIRECTORY = "./store/";

    shellHook = ''
      export PATH="target/debug/:$PATH";
    '';
  };
} 
