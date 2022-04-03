# Centrifuge - FUDGE 

FUDGE - FUlly Decoupled Generic Environment

:warning: NOTE:

:warning: Expect Breaking Changes

:warning: Expect reaching not yet implemented errors during runtime, and various bugs during altering a chains state as interacting with the underlying db is complex. Expect errors due to unwrapping where error handling should instead propagate errors. 

FUDGE provides a core lib for interacting with a substrate based blockchain database.
This allows users to 
* Load a database and do analytics on any given block the database provides 
* Populate an empty database with a genesis configuration
* Manipulate the latest state of the database arbitrarily
* Provides externalities
* Build blocks on top of the latest block in the database (in WASM environment)
* Build blocks in companion with a relay-chain (i.e. mimics normal relay-chain-parachain environment locally)


### Features
- [x] Accessing state of a chain at any given block (given the database contains that state)
- [x] Mutating the latest state of a given chain
- [x] Executing runtime code in any given state of the chain
- [x] Building blocks for Para- Relay-Chain setups, Standalone-chains
  
  (**NOTE:** Does **NOT** mimic actual block importing procedure in the para-relay-chain-setup. Instead forces head of parachains manually)

  (**NOTE:** Does build a block with the normal block building procedure that standard substrate node use)
  - [ ] Mimic block production in para-relay-chain-setup
- [ ] Inject extrinsics into dummy-pool, that provides them to the block building mechanism (Currently only directly via builders)
- [ ] XCM-Support
- [ ] Mutating past state and propagating changes to the latest block


## How to use?
Add the following dependecy to your project (*Currently only polkadot-v0.9.17 compatability*). Please always use 
the latest version available until we reach a more stable *0.1.x* state.
```toml
fudge = { git = "https://github.com/centrifuge/fudge", tag = "v0.0.5-polkadot-v0.9.17"}
```

Set up your test environment like the following.
In this setup the `RelayCidp` and `Centrifuge` are the `CreateInherentDataProviders` of the respective chains. 

For a detailed example please look at test cases in the repo at *crate::core::tests*. 
```rust
#[fudge::companion]
pub struct TestEnv {
	#[fudge::relaychain]
	relay: RelaychainBuilder<RelayBlock, RelayRtApi, RelayRt, RelayCidp, Dp>,
	#[fudge::parachain(2000)]
	centrifuge: ParachainBuilder<CentrifugeBlock, CentrifugeRtApi, CentrifugeCidp, Dp>,
}
```
The `TestEnv` will expose the following methods:
```rust

impl TestEnv {
  // Provides the latest state of the chain. (Externalities provided environment)
  pub fn with_state(chain: Chain, exec: FnOnce() -> Result<R, E>) -> Result<(), ()>;

  // Provides the latest state of the chain mutably. (Externalities provided environment)
  pub fn with_mut_state(chain: Chain, exec: FnOnce() -> Result<R, E>) -> Result<(), ()>;

  // Builds one parachain block and two relay-chain blocks
  pub fn evolve() -> Result<(), ()>;
}
```

that can be called like
```rust
// Access the state of the parachain
env.with_state(Chain::Para(ID_HERE), || {
    // Externalities provided environment
    // at the latest state of the chain
})

// Access the state of the relay, with mutations 
// being stored, hence altering the latest state of the parachain
env.with_mut_state(Chain::Relay, || {
// Externalities provided environment
// at the latest state of the chain
})

// Progess the chains
env.evolve()
```


