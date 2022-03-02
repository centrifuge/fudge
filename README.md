# Centrifuge - FUDGE 

FUDGE - FUlly Decoupled Generic Environment.
(Note: Let's face it. I had the name and needed it to be some acronym.)

FUDGE provides a core lib for interacting with a substrate based blockchain database.
This allows users to 
* Load a database and do analytics on any given block the database provides 
* Populate an empty database with a genesis configuration
* Manipulate the latest state of the database arbitrarily
* Provides externalities
* Build blocks on top of the latest block in the database (in WASM environment)
* Build blocks in companion with a relay-chain (i.e. mimics normal relay-chain-parachain environment locally)



**NOTE: The state described here DOES NOT describe the current state of the lib. Please, go to 
*docs/current_state.md* in order to check what already has been implemented.**

## How to use?


