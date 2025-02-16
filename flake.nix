{
  description = "A flake for AWS lambda Rust development";

  inputs.nixpkgs.url = "nixpkgs/nixos-unstable";

  outputs = { self, nixpkgs }: {
    devShell.x86_64-linux = let
      system = "x86_64-linux";
      pkgs = import nixpkgs { inherit system; };
    in pkgs.mkShell rec{
      buildInputs = [
        pkgs.rustc
        pkgs.cargo
        pkgs.cargo-watch
        pkgs.aws-sam-cli
        pkgs.awscli2
        pkgs.cargo-lambda
        pkgs.openssl
        pkgs.pkg-config
        pkgs.taglib
        pkgs.git
        pkgs.libxml2
        pkgs.libxslt
        pkgs.libzip
        pkgs.zlib
      ];

    };
  };
}
