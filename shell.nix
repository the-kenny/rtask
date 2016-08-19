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
    RTASK_DIRECTORY = "./store/";

    shellHook = ''
      export PATH="target/debug/:$PATH";
    '';
  };
} 
