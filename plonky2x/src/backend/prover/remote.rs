use core::time::Duration;
use std::env;

use futures::future::join_all;
use log::{debug, info};
use plonky2::field::extension::Extendable;
use plonky2::hash::hash_types::RichField;
use plonky2::plonk::config::{AlgebraicHasher, GenericConfig};
use plonky2::plonk::proof::ProofWithPublicInputs;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tokio::time::sleep;

use super::service::{GetProofResponse, ProvingService};
use super::Prover;
use crate::backend::circuit::io::{CircuitInput, CircuitOutput, ProofWithPublicInputsSerializable};
use crate::backend::circuit::Circuit;
// use crate::mapreduce::serialize::CircuitDataSerializable;

#[derive(Serialize, Deserialize, Debug)]
pub struct ContextData {
    pub circuit_id: String,
    pub input: Vec<String>,
    pub tag: String,
}

/// A prover that uses the Succinct remote prover to generate proofs. The built circuit must
/// already be uploaded to Succinct and be referenced via the enviroment variable `RELEASE_ID`.
pub struct RemoteProver {
    pub client: Client,
}

impl Prover for RemoteProver {
    fn new() -> Self {
        Self {
            client: Client::new(),
        }
    }

    async fn prove<F, C, const D: usize>(
        &self,
        circuit: &Circuit<F, C, D>,
        input: &CircuitInput<F, D>,
    ) -> (ProofWithPublicInputs<F, C, D>, CircuitOutput<F, D>)
    where
        F: RichField + Extendable<D>,
        C: GenericConfig<D, F = F> + 'static,
        C::Hasher: AlgebraicHasher<F>,
    {
        // Calculate create proof payload.
        let release_id = env::var("RELEASE_ID").expect("enviroment variable RELEASE_ID is not set");
        let circuit_id = circuit.id();
        let context = serde_json::to_string_pretty(&ContextData {
            circuit_id: circuit_id.clone(),
            input: input.buffer.iter().map(|x| x.to_string()).collect(),
            tag: "map".to_string(),
        })
        .unwrap();

        // Call the service to create a proof.
        let service = ProvingService::new();
        let proof_id = service
            .create_proof(release_id, "0x".to_string(), context)
            .await;

        /// Wait up to 120 seconds for the proof to finish generating.
        const MAX_RETRIES: usize = 120;
        let mut response: GetProofResponse = GetProofResponse {
            id: "".to_string(),
            status: "".to_string(),
            result: None,
        };
        for _ in 0..MAX_RETRIES {
            response = service.get_proof(proof_id.clone()).await;
            if response.status == "success" {
                break;
            } else if response.status == "failure" {
                panic!("proof generation failed proof_id={}", response.id);
            }
            sleep(Duration::from_secs(1)).await;
            debug!("Waiting for proof to generate proof_id={}", response.id);
        }

        // Check if the proof was generated successfully.
        if response.status != "success" {
            panic!("proof generation timed out proof_id={}", response.id);
        }
        info!("Proof generated successfully proof_id={}", response.id);

        // Deserialize the proof.
        let result = response.result;
        let proof = ProofWithPublicInputs::<F, C, D>::deserialize_from_json(
            result.clone().unwrap().get("proof").unwrap().to_owned(),
        );
        let output = CircuitOutput::<F, D>::deserialize_from_json(
            circuit,
            result.unwrap().get("output").unwrap().to_owned(),
        );
        (proof, output)
    }

    async fn prove_batch<F, C, const D: usize>(
        &self,
        circuit: &Circuit<F, C, D>,
        inputs: Vec<CircuitInput<F, D>>,
    ) -> Vec<(ProofWithPublicInputs<F, C, D>, CircuitOutput<F, D>)>
    where
        F: RichField + Extendable<D>,
        C: GenericConfig<D, F = F> + 'static,
        C::Hasher: AlgebraicHasher<F>,
    {
        let mut futures = Vec::new();
        for i in 0..inputs.len() {
            info!("Starting proof {}/{}.", i + 1, inputs.len());
            let future = self.prove(circuit, &inputs[i]);
            futures.push(future);
        }
        join_all(futures).await
    }
}
