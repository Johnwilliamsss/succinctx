package main

import (
	_ "embed"
	"flag"
	"os"

	"github.com/consensys/gnark/logger"
	"github.com/succinctlabs/succinctx/plonky2x/verifier/system"
)

func main() {
	// https://github.com/succinctlabs/gnark-plonky2-verifier/blob/c01f530fe1d0107cc20da226cfec541ece9fb882/goldilocks/base.go#L131
	os.Setenv("USE_BIT_DECOMPOSITION_RANGE_CHECK", "true")

	circuitPath := flag.String("circuit", "", "circuit data directory")
	dataPath := flag.String("data", "", "data directory")
	proofFlag := flag.Bool("prove", false, "create a proof")
	verifyFlag := flag.Bool("verify", false, "verify a proof")
	compileFlag := flag.Bool("compile", false, "compile and save the universal verifier circuit")
	exportFlag := flag.Bool("export", false, "export the Solidity verifier")
	systemFlag := flag.String("system", "groth16", "proving system to use (groth16, plonk)")
	flag.Parse()

	logger := logger.Logger()

	if *dataPath == "" {
		logger.Error().Msg("please specify a path to data dir (where the compiled gnark circuit data will be)")
		os.Exit(1)
	}
	logger.Info().Msg("Circuit path: " + *circuitPath)
	logger.Info().Msg("Data path: " + *dataPath)

	var s system.ProvingSystem
	if *systemFlag == "groth16" {
		s = system.NewGroth16System(logger, *circuitPath, *dataPath)
	} else if *systemFlag == "plonk" {
		s = system.NewPlonkSystem(logger, *circuitPath, *dataPath)
	} else {
		logger.Error().Msg("invalid proving system")
		os.Exit(1)
	}

	if *compileFlag {
		err := s.Compile()
		if err != nil {
			logger.Error().Msg("failed to compile circuit:" + err.Error())
			os.Exit(1)
		}
	}

	if *proofFlag {
		err := s.Prove()
		if err != nil {
			logger.Error().Msg("failed to create proof:" + err.Error())
			os.Exit(1)
		}
	}

	if *verifyFlag {
		err := s.Verify()
		if err != nil {
			logger.Error().Msg("failed to verify proof:" + err.Error())
			os.Exit(1)
		}
	}

	if *exportFlag {
		err := s.Export()
		if err != nil {
			logger.Error().Msg("failed to export verifier:" + err.Error())
			os.Exit(1)
		}
	}
}
