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

//! Inherent data providers that should only be used within FUDGE

use sp_core::sp_std::sync::Arc;
use sp_inherents::{InherentData, InherentIdentifier};
use sp_std::collections::btree_map::BTreeMap;
use sp_std::sync::Mutex;
use sp_std::time::Duration;
use sp_timestamp::{InherentError, INHERENT_IDENTIFIER};

lazy_static::lazy_static!(
	pub static ref INSTANCES: Arc<Mutex<BTreeMap<Instance, CurrTimeProvider>>> = Arc::new(Mutex::new(BTreeMap::new()));
);

pub type Instance = u64;
pub type Ticks = u128;

#[derive(Clone)]
pub struct CurrTimeProvider {
	instance: Instance,
	start: Duration,
	delta: Duration,
	ticks: Ticks,
}

impl CurrTimeProvider {
	pub fn new(instance: Instance, delta: Duration, start: Option<Duration>) -> Self {
		let storage = INSTANCES.clone();
		let mut locked_instances = storage.lock().expect("Time MUST NOT fail.");

		let start = if let Some(start) = start {
			start
		} else {
			let now = std::time::SystemTime::now();
			let dur = now
				.duration_since(std::time::SystemTime::UNIX_EPOCH)
				.expect("Current time is always after unix epoch; qed");

			dur
		};

		locked_instances
			.entry(instance)
			.or_insert(CurrTimeProvider {
				instance,
				start,
				delta,
				ticks: 0,
			})
			.clone()
	}

	pub fn get_instance(instance: Instance) -> Option<Self> {
		let storage = INSTANCES.clone();
		let locked_instances = storage.lock().expect("Time MUST NOT fail.");

		locked_instances
			.get(&instance)
			.map(|instance| instance.clone())
	}

	fn update_time(&self) {
		let storage = INSTANCES.clone();
		let mut instances = storage.lock().expect("Time MUST NOT fail.");
		let instance = instances
			.get_mut(&self.instance)
			.expect("ONLY calls this method after new(). qed");
		instance.ticks = instance.ticks + 1;
	}

	pub fn current_time(&self) -> sp_timestamp::Timestamp {
		let delta: u128 = self.delta.as_millis() * self.ticks;
		let timestamp: u128 = self.start.as_millis() + delta;
		sp_timestamp::Timestamp::new(timestamp as u64)
	}
}

#[async_trait::async_trait]
impl sp_inherents::InherentDataProvider for CurrTimeProvider {
	fn provide_inherent_data(
		&self,
		inherent_data: &mut InherentData,
	) -> Result<(), sp_inherents::Error> {
		inherent_data
			.put_data(INHERENT_IDENTIFIER, &self.current_time())
			.unwrap();
		self.update_time();
		Ok(())
	}

	async fn try_handle_error(
		&self,
		identifier: &InherentIdentifier,
		error: &[u8],
	) -> Option<Result<(), sp_inherents::Error>> {
		if *identifier != INHERENT_IDENTIFIER {
			return None;
		}

		match InherentError::try_from(&INHERENT_IDENTIFIER, error)? {
			InherentError::ValidAtTimestamp(_valid) => {
				/*
				let max_drift = self.max_drift;
				let timestamp = self.timestamp;
				// halt import until timestamp is valid.
				// reject when too far ahead.
				if valid > timestamp + max_drift {
					return Some(Err(sp_inherents::Error::Application(Box::from(
						InherentError::TooFarInFuture,
					))))
				}

				let diff = valid.checked_sub(timestamp).unwrap_or_default();
				log::info!(
					target: "timestamp",
					"halting for block {} milliseconds in the future",
					diff.0,
				);

				futures_timer::Delay::new(diff.as_duration()).await;
				*/
				// TODO: Sane?
				Some(Ok(()))
			}
			o => Some(Err(sp_inherents::Error::Application(Box::from(o)))),
		}
	}
}
