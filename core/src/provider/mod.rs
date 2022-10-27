// Copyright 2021 Centrifuge Foundation (centrifuge.io).
//
// This file is part of the FUDGE project.
// FUDGE is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version (see http://www.gnu.org/licenses).
// Centrifuge is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

pub use externalities_provider::ExternalitiesProvider;
use sc_executor::RuntimeVersionOf;
use sc_service::{
	config::ExecutionStrategies, ClientConfig, Configuration, KeystoreContainer, TFullBackend,
	TFullCallExecutor, TFullClient, TaskManager,
};
use sp_api::{BlockT, ConstructRuntimeApi};
use sp_core::traits::{CodeExecutor, SpawnNamed};
use sp_keystore::SyncCryptoStorePtr;
use sp_runtime::BuildStorage;
use sp_std::{marker::PhantomData, str::FromStr, sync::Arc};
use sp_storage::Storage;

pub use crate::provider::state_provider::DbOpen;
use crate::provider::state_provider::StateProvider;

mod externalities;
mod initiator;
mod state;

pub trait Initiator {}

pub trait GenesisState {}
