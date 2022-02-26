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

use sp_inherents::{InherentData, InherentIdentifier};
use sp_runtime::SaturatedConversion;
use sp_std::collections::btree_map::BTreeMap;
use sp_std::time::Duration;
use sp_timestamp::{InherentError, INHERENT_IDENTIFIER};

static mut INSTANCES: *mut BTreeMap<Instance, CurrTimeProvider> =
	0usize as *mut BTreeMap<Instance, CurrTimeProvider>;

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
	pub fn get_instance(instance: Instance) -> Self {
		let instances = unsafe {
			if INSTANCES.is_null() {
				let pt = Box::into_raw(Box::new(BTreeMap::<Instance, CurrTimeProvider>::new()));
				INSTANCES = pt;

				&mut *INSTANCES
			} else {
				&mut *INSTANCES
			}
		};

		instances.get(&instance).unwrap().clone()
	}

	pub fn new(instance: Instance, delta: Duration, start: Option<Duration>) -> Self {
		let instances = unsafe {
			if INSTANCES.is_null() {
				let pt = Box::into_raw(Box::new(BTreeMap::<Instance, CurrTimeProvider>::new()));
				INSTANCES = pt;

				&mut *INSTANCES
			} else {
				&mut *INSTANCES
			}
		};

		let start = if let Some(start) = start {
			start
		} else {
			let now = std::time::SystemTime::now();
			let dur = now
				.duration_since(std::time::SystemTime::UNIX_EPOCH)
				.expect("Current time is always after unix epoch; qed");

			dur
		};

		instances
			.entry(instance)
			.or_insert(CurrTimeProvider {
				instance,
				start,
				delta,
				ticks: 0,
			})
			.clone()
	}

	pub fn update_time(&self) {
		let instances = unsafe { &mut *INSTANCES };
		instances.insert(
			self.instance,
			CurrTimeProvider {
				start: self.start,
				instance: self.instance,
				delta: self.delta,
				ticks: self.ticks + 1,
			},
		);
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
