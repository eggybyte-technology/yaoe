{
  description = "YAOE local dev shell";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    fenix.url = "github:nix-community/fenix";
  };

  outputs =
    {
      nixpkgs,
      flake-utils,
      fenix,
      ...
    }:
    let
      constantsRs = builtins.readFile ./crates/yaoe-home/src/constants.rs;
      constant =
        name:
        let
          prefix = "pub const ${name}: &str = \"";
          lines = builtins.split "\n" constantsRs;
          matches = builtins.filter (
            line: builtins.isString line && builtins.match "${prefix}.*" line != null
          ) lines;
          line =
            if matches == [ ] then
              throw "missing ${name} in crates/yaoe-home/src/constants.rs"
            else
              builtins.elemAt matches 0;
          start = builtins.stringLength prefix;
          len = builtins.stringLength line - start - 2;
        in
        builtins.substring start len line;
      singBoxVersion = constant "SING_BOX_VERSION";
    in
    flake-utils.lib.eachSystem [ "x86_64-linux" "aarch64-linux" ] (
      system:
      let
        pkgs = import nixpkgs { inherit system; };

        rustToolchainToml = builtins.fromTOML (builtins.readFile ./rust-toolchain.toml);
        rustToolchain = fenix.packages.${system}.toolchainOf {
          channel = rustToolchainToml.toolchain.channel;
          sha256 = "sha256-mvUGEOHYJpn3ikC5hckneuGixaC+yGrkMM/liDIDgoU=";
        };
        rust = fenix.packages.${system}.combine [
          rustToolchain.cargo
          rustToolchain.rustc
          rustToolchain.rustfmt
          rustToolchain.clippy
          rustToolchain.rust-std
          fenix.packages.${system}.rust-analyzer
        ];

        singBoxTarball = {
          x86_64-linux = {
            suffix = "amd64";
            hash = "sha256-19NbgG23Ls+zAhohSKty8D6K8LqlBnqF1Vc4iFdP2JA=";
          };
          aarch64-linux = {
            suffix = "arm64";
            sha256 = "18fnsks1mla40s3di8jr7sy8z8xp39jakq6agl6i8v8kby1lyikc";
          };
        }.${system};

        mihomoAsset = {
          x86_64-linux = {
            suffix = "amd64";
            hash = "sha256-+z40xVhE84n/VGeeWjrsMx1ew4AGwg+NzEdvtHdopY8=";
          };
          aarch64-linux = {
            suffix = "arm64";
            hash = "sha256-h9sMZmCpVXqQG1dQ+ZeWfnHYwK8H6h0d1NBMKNp/fm8=";
          };
        }.${system};

        singBox = pkgs.stdenvNoCC.mkDerivation {
          pname = "sing-box";
          version = singBoxVersion;
          src = pkgs.fetchurl (
            {
              url = "https://github.com/SagerNet/sing-box/releases/download/v${singBoxVersion}/sing-box-${singBoxVersion}-linux-${singBoxTarball.suffix}.tar.gz";
            }
            // (
              if singBoxTarball ? hash then
                { inherit (singBoxTarball) hash; }
              else
                { inherit (singBoxTarball) sha256; }
            )
          );
          sourceRoot = ".";
          installPhase = ''
            runHook preInstall
            install -Dm755 sing-box-${singBoxVersion}-linux-${singBoxTarball.suffix}/sing-box $out/bin/sing-box
            runHook postInstall
          '';
        };

        mihomo = pkgs.stdenvNoCC.mkDerivation {
          pname = "mihomo";
          version = "1.19.27";
          src = pkgs.fetchurl {
            url = "https://github.com/MetaCubeX/mihomo/releases/download/v1.19.27/mihomo-linux-${mihomoAsset.suffix}-v1.19.27.gz";
            inherit (mihomoAsset) hash;
          };
          dontUnpack = true;
          nativeBuildInputs = [ pkgs.gzip ];
          installPhase = ''
            runHook preInstall
            mkdir -p $out/bin
            gzip -dc $src > $out/bin/mihomo
            chmod 0755 $out/bin/mihomo
            runHook postInstall
          '';
        };
      in
      {
        devShells.default = pkgs.mkShell {
          packages = [
            rust
            pkgs.cargo-nextest
            pkgs.curl
            pkgs.diffutils
            pkgs.git
            pkgs.openssh
            pkgs.gnutar
            pkgs.gzip
            pkgs.unzip
            pkgs.jq
            singBox
            mihomo
            pkgs.wrangler
          ];

          shellHook = ''
            export PATH="$HOME/.cargo/bin:$PATH"
          '';
        };
      }
    );
}
