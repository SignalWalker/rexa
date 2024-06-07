{
  description = "";
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    alejandra = {
      url = "github:kamadorueda/alejandra";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    ocapn-test-suite = {
      url = "github:ocapn/ocapn-test-suite";
      flake = false;
    };
  };
  outputs = inputs @ {
    self,
    nixpkgs,
    ...
  }:
    with builtins; let
      std = nixpkgs.lib;
      systems = ["x86_64-linux"];
      nixpkgsFor = std.genAttrs systems (system:
        import nixpkgs {
          localSystem = builtins.currentSystem or system;
          crossSystem = system;
          overlays = [];
        });
    in {
      formatter = std.mapAttrs (system: pkgs: pkgs.default) inputs.alejandra.packages;
      devShells = std.genAttrs systems (system: let
        pkgs = nixpkgsFor.${system};
      in {
        default = pkgs.mkShell (let
          python = pkgs.python311.withPackages (ps:
            with ps; [
              cryptography
              stem
            ]);
        in {
          nativeBuildInputs = [
            pkgs.pkg-config
          ];
          buildInputs = [
            pkgs.sqlite
            pkgs.openssl
            pkgs.zlib
          ];
          packages = [
            python
            pkgs.tor
          ];

          env.PKG_CONFIG_PATH = "${pkgs.openssl.dev}/lib/pkgconfig:${pkgs.sqlite.dev}/lib/pkgconfig";

          env.REXA_OCAPN_TEST_SUITE_DIR = toString inputs.ocapn-test-suite;
          env.REXA_PYTHON_PATH = "${python}/bin/python";

          LD_LIBRARY_PATH = std.concatStringsSep ":" ["${pkgs.sqlite.out}/lib" "${pkgs.openssl.out}/lib"];
        });
      });
    };
}
# {
#   description = "";
#   inputs = {
#     nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
#     alejandra = {
#       url = "github:kamadorueda/alejandra";
#       inputs.nixpkgs.follows = "nixpkgs";
#     };
#     crane = {
#       url = "github:ipetkov/crane";
#       inputs.nixpkgs.follows = "nixpkgs";
#     };
#     fenix = {
#       url = "github:nix-community/fenix";
#       inputs.nixpkgs.follows = "nixpkgs";
#       inputs.rust-analyzer-src.follows = "";
#     };
#     advisory-db = {
#       url = "github:rustsec/advisory-db";
#       flake = false;
#     };
#     rust-overlay = {
#       url = "github:oxalica/rust-overlay";
#       inputs.nixpkgs.follows = "nixpkgs";
#     };
#     ocapn-test-suite = {
#       url = "github:ocapn/ocapn-test-suite";
#       flake = false;
#     };
#   };
#   outputs = inputs @ {
#     self,
#     nixpkgs,
#     ...
#   }:
#     with builtins; let
#       std = nixpkgs.lib;
#
#       systems = attrNames inputs.crane.lib;
#       nixpkgsFor = std.genAttrs systems (system:
#         import nixpkgs {
#           localSystem = builtins.currentSystem or system;
#           crossSystem = system;
#           overlays = [inputs.rust-overlay.overlays.default];
#         });
#
#       toolchainToml = fromTOML (readFile ./rust-toolchain.toml);
#       toolchainFor = std.mapAttrs (system: pkgs: pkgs.rust-bin.fromRustupToolchain toolchainToml.toolchain) nixpkgsFor;
#
#       craneFor = std.mapAttrs (system: pkgs: (inputs.crane.mkLib pkgs).overrideToolchain toolchainFor.${system}) nixpkgsFor;
#
#       commonArgsFor =
#         std.mapAttrs (system: pkgs: let
#           crane = craneFor.${system};
#         in {
#           src = crane.cleanCargoSource (crane.path ./.);
#           strictDeps = true;
#           nativeBuildInputs = with pkgs; [
#             llvmPackages_16.clang
#             mold
#             pkg-config
#           ];
#           buildInputs = with pkgs; [
#             sqlite
#             openssl
#             zlib
#           ];
#         })
#         nixpkgsFor;
#
#       cargoToml = fromTOML (readFile ./Cargo.toml);
#       name = cargoToml.package.metadata.crane.name or cargoToml.package.name or cargoToml.workspace.metadata.crane.name;
#       version = cargoToml.package.version or cargoToml.workspace.package.version;
#     in {
#       formatter = std.mapAttrs (system: pkgs: pkgs.default) inputs.alejandra.packages;
#       packages =
#         std.mapAttrs (system: pkgs: let
#           crane = craneFor.${system};
#           src = crane.cleanCargoSource (crane.path ./.);
#         in {
#           default = self.packages.${system}.${name};
#           "${name}-artifacts" = crane.buildDepsOnly commonArgsFor.${system};
#           ${name} = crane.buildPackage (commonArgsFor.${system}
#             // {
#               cargoArtifacts = self.packages.${system}."${name}-artifacts";
#             });
#         })
#         nixpkgsFor;
#       checks =
#         std.mapAttrs (system: pkgs: let
#           crane = craneFor.${system};
#           commonArgs = commonArgsFor.${system};
#           cargoArtifacts = self.packages.${system}."${name}-artifacts";
#         in {
#           ${name} = pkgs.${name};
#           "${name}-clippy" = crane.cargoClippy (commonArgs
#             // {
#               inherit cargoArtifacts;
#             });
#           "${name}-coverage" = crane.cargoTarpaulin (commonArgs
#             // {
#               inherit cargoArtifacts;
#             });
#           "${name}-audit" = crane.cargoAudit (commonArgs
#             // {
#               pname = name;
#               inherit version;
#               inherit cargoArtifacts;
#               advisory-db = inputs.advisory-db;
#             });
#           "${name}-deny" = crane.cargoDeny (commonArgs
#             // {
#               inherit cargoArtifacts;
#             });
#         })
#         self.packages;
#       devShells =
#         std.mapAttrs (system: pkgs: let
#           selfPkgs = self.packages.${system};
#           toolchain = toolchainFor.${system}.override {
#             extensions = [
#               "rust-analyzer"
#               "rustfmt"
#               "clippy"
#             ];
#           };
#           crane = (inputs.crane.mkLib pkgs).overrideToolchain toolchain;
#
#           python = pkgs.python311.withPackages (ps:
#             with ps; [
#               cryptography
#               stem
#             ]);
#         in {
#           ${name} = crane.devShell {
#             checks = self.checks.${system};
#             packages =
#               [
#                 python
#                 pkgs.tor
#               ]
#               ++ (with pkgs; [
#                 cargo-audit
#                 cargo-license
#                 cargo-dist
#               ]);
#             shellHook = let
#               extraLdPaths =
#                 pkgs.lib.makeLibraryPath (with pkgs; [
#                   ]);
#             in ''
#               export LD_LIBRARY_PATH="${extraLdPaths}:$LD_LIBRARY_PATH"
#             '';
#             env.REXA_OCAPN_TEST_SUITE_DIR = toString inputs.ocapn-test-suite;
#             env.REXA_PYTHON_PATH = "${python}/bin/python";
#           };
#           default = self.devShells.${system}.${name};
#         })
#         nixpkgsFor;
#     };
# }

