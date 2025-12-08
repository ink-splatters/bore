{
  perSystem = {config, ...}: let
    inherit (config) craneLib src commonArgs cargoArtifacts;
  in {
    checks = {
      inherit (config.packages) bore;

      bore-clippy = craneLib.cargoClippy (
        commonArgs
        // {
          inherit cargoArtifacts;
          cargoClippyExtraArgs = "--all-targets -- --deny warnings";
        }
      );

      bore-doc = craneLib.cargoDoc (
        commonArgs
        // {
          inherit cargoArtifacts;
        }
      );

      bore-fmt = craneLib.cargoFmt {
        inherit src;
      };

      bore-nextest = craneLib.cargoNextest (
        commonArgs
        // {
          inherit cargoArtifacts;
          partitions = 1;
          partitionType = "count";
        }
      );
    };
  };
}
