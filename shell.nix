{ nixpkgs ? <nixpkgs> }:
let
  pkgs = import nixpkgs {};
in rec {
  rustEnv = pkgs.stdenv.mkDerivation {
    name = "rust";
    version = "1.2.3.4";
    src = ./.;
    buildInputs = with pkgs; [ rustc cargo pkgconfig sqlite ];

    RUST_LOG="rtask=info";
    RUST_BACKTRACE=1;

    RTASK_DIRECTORY = "./store/";
    RUST_SRC_PATH="${pkgs.rustc.src}";

    shellHook = ''
      export PATH="target/debug/:$PATH";
    '';
  };
} 
