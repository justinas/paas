{ pkgs ? import <nixpkgs> { } }:
pkgs.mkShell {
  buildInputs = with pkgs; [ cargo openssl protobuf rustc ];
  shellHook = ''
    export PROTOBUF_LOCATION=${pkgs.protobuf}
    export PROTOC=${pkgs.protobuf}/bin/protoc
    export PROTOC_INCLUDE=${pkgs.protobuf}/include
  '';
}
