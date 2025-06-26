# Proof Generator

## Setup

1. Install scarb: <https://docs.swmansion.com/scarb/> (using asdf)

2. Cairo runner is an interpreter that produces all the necessary artifacts for the proving.

    ```sh
    cargo install --git https://github.com/lambdaclass/cairo-vm cairo1-run
    ```

3. Integrity Serializer

Integrity is a set of Cairo contracts and a toolchain for recursive proving on Starknet.

    ```sh
        cargo install --git https://github.com/HerodotusDev/integrity-calldata-generator swiftness
    ```

1. Install Air Provers
    - For the full stone prover
        `docker pull ghcr.io/dipdup-io/stone-packaging/stone-prover:master`
    - For the CPU Air Prover (Lightweight):
        `docker pull ghcr.io/dipdup-io/stone-packaging/cpu_air_prover:master`
    - For the CPU Air Verifier (Lightweight):
        `docker pull ghcr.io/dipdup-io/stone-packaging/cpu_air_verifier:master`

2. Build: `scarb build`.

3. Our output file is `target/release/cairo1.sierra.json`

### Run and get execution artifacts

Now we can use `cairo1-run` runner to produce execution trace, we just need to provide the compiled program (Sierra file) and serialized arguments — 4 field elements, where first felt is the private key, and the rest are the message.

```
cairo1-run target/release/cairo1.sierra.json \
    --layout recursive_with_poseidon \
    --arguments-file input.cairo1.json \
    --proof_mode \
    --air_public_input target/public_input.json \
    --air_private_input target/private_input.json \
    --trace_file target/trace \
    --memory_file target/memory
```

### Generate proof

Given the execution artifacts and predefined prover configuration (see https://stone-packaging.pages.dev/usage/configuration for more information) we can generate a STARK proof for this concrete program invocation.

```
cpu_air_prover \
    --parameter_file prover_params.json \
    --prover_config_file prover_config.json \
    --private_input_file target/private_input.json \
    --public_input_file target/public_input.json \
    --out_file target/proof.json \
    --generate_annotations true
```

We can check that the proof is correct (locally):

```
cpu_air_verifier --in_file target/proof.json && echo "Proof is valid!"
```

### Serialize and split the proof

The obtained proof is pretty large and it's serialized in JSON, which is not suitable for submitting onchain. So before all we need to encode the proof data and split into several digestible parts so that we remain within gas limits for every submitted transaction. Swiftness utility does exactly that, we should provide some extra parameters to specify the proving options we use: layout (set of builtins), commitment hash function, and prover version.

```
rm -rf ./target/calldata
mkdir ./target/calldata

# see https://github.com/HerodotusDev/integrity/blob/main/deployed_contracts.md
echo "0x16409cfef9b6c3e6002133b61c59d09484594b37b8e4daef7dcba5495a0ef1a" > ./target/calldata/contract_address

swiftness --proof target/proof.json \
    --layout recursive_with_poseidon \
    --hasher keccak_160_lsb \
    --stone-version stone6 \
    --out target/calldata
```

### Verify proof on Starknet

Now we can verify the split proof on Starket using Integrity contracts. We need to provide a unique job ID so that the verifier contract can keep track of the multiple submissions.

```
JOB_ID=$((RANDOM % 10000 + 1)) && ./scripts/register_fact.sh $JOB_ID recursive_with_poseidon keccak_160_lsb stone6 cairo1
```

### Check the verification fact

Once the proof is verified we should be able to query the status of the verification fact. In order to do that we need to calculate the fact ID which is a hash of the program and execution output. Integrity provides a nice visual tool where you can upload your proof and get the fact hash: https://integrity-hashes-calculator.vercel.app/

Then we can go to the explorer, open the fact registry contract (there is a separate contract for each proving configuration, check here https://github.com/HerodotusDev/integrity/blob/main/deployed_contracts.md) and navigate to the "Read Contract" tab.

For example:
- Open https://sepolia.voyager.online/contract/0x16409cfef9b6c3e6002133b61c59d09484594b37b8e4daef7dcba5495a0ef1a#readContract
- The fact hash is `0x6d9ec29a2511b606d75d1094b0719d2af6136e0ac89d214e1b0a18a711fb562`
- Query `get_all_verifications_for_fact_hash`

We will see
```json
[
    {
        "verification_hash": "0x0606d88e8e6983c4d6f31006c376ea3d890a2c32849420440f8e72023c314a2f",
        "security_bits": "0x3c",
        "verifier_config": {
            "layout": "0x7265637572736976655f776974685f706f736569646f6e",
            "hasher": "0x6b656363616b5f3136305f6c7362",
            "stone_version": "0x73746f6e6536",
            "memory_verification": "0x636169726f31"
        }
    }
]
```*