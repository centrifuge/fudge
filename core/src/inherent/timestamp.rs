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
use sp_runtime::SaturatedConversion;
use sp_std::collections::btree_map::BTreeMap;
use sp_std::sync::Mutex;
use sp_std::time::Duration;
use sp_timestamp::{InherentError, INHERENT_IDENTIFIER};

lazy_static::lazy_static!(
	pub static ref INSTANCES: Arc<Mutex<BTreeMap<Instance, CurrTimeProvider>>> = Arc::new(Mutex::new(BTreeMap::new()));
);

pub type Instance = u64;
pub type Ticks = u128;

#[derive(Clone, Debug)]
pub struct CurrTimeProvider {
	instance: Instance,
	start: Duration,
	delta: Duration,
	ticks: Ticks,
}

impl CurrTimeProvider {
	/// Creates a new instance and returns the **old** instance of it was existing.
	///
	/// To get the actual instance please use `CurrTimeProvider::get_instance()`.
	pub fn new(instance: Instance, delta: Duration, start: Option<Duration>) -> Option<Self> {
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

		let prev = locked_instances.insert(
			instance,
			CurrTimeProvider {
				instance,
				start,
				delta,
				ticks: 0,
			},
		);

		if let Some(ref old_instance) = prev {
			tracing::event!(
				tracing::Level::WARN,
				"Overwriting time-instance. Previous instance was {:?}.",
				old_instance
			);
		}

		prev
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
		let delta: u128 = self.delta.as_millis().saturating_mul(self.ticks);
		let timestamp: u128 = self.start.as_millis().saturating_add(delta);
		sp_timestamp::Timestamp::new(timestamp.saturated_into::<u64>())
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
			InherentError::ValidAtTimestamp(_valid) => Some(Ok(())),
			o => Some(Err(sp_inherents::Error::Application(Box::from(o)))),
		}
	}
}

#[cfg(test)]
mod test {
	use super::*;

	/// Sat Jan 01 2022 00:00:00 GMT+0000
	pub const START_DATE: u64 = 1640995200u64;

	#[test]
	fn steps_work_correctly() {
		const DELTA: u64 = 12u64;

		let delta = Duration::from_secs(DELTA);
		CurrTimeProvider::new(0, delta, Some(Duration::from_secs(START_DATE)));
		let time = CurrTimeProvider::get_instance(0).expect("Instance is initialized. qed");

		assert_eq!(
			time.current_time().as_duration().as_secs() as u64,
			START_DATE
		);

		// Progress time by delta
		time.update_time();
		let time = CurrTimeProvider::get_instance(0).expect("Instance is initialized. qed");
		assert_eq!(
			time.current_time().as_duration().as_secs() as u64,
			START_DATE + delta.as_secs() as u64
		);

		// Progress time by delta
		time.update_time();
		let time = CurrTimeProvider::get_instance(0).expect("Instance is initialized. qed");
		assert_eq!(
			time.current_time().as_duration().as_secs() as u64,
			START_DATE + 2 * delta.as_secs() as u64
		);
	}

	#[test]
	fn instances_work_correctly() {
		const DELTA_A: u64 = 12u64;
		const DELTA_B: u64 = 6u64;

		let delta_a = Duration::from_secs(DELTA_A);
		CurrTimeProvider::new(3, delta_a, Some(Duration::from_secs(START_DATE)));
		let time_a = CurrTimeProvider::get_instance(4).expect("Instance is initialized. qed");

		let delta_b = Duration::from_secs(DELTA_A);
		CurrTimeProvider::new(4, delta_b, Some(Duration::from_secs(START_DATE)));
		let time_b = CurrTimeProvider::get_instance(3).expect("Instance is initialized. qed");

		assert_eq!(
			time_a.current_time().as_duration().as_secs() as u64,
			START_DATE
		);
		assert_eq!(
			time_b.current_time().as_duration().as_secs() as u64,
			START_DATE
		);

		// Progress time by delta
		time_a.update_time();
		let time_a = CurrTimeProvider::get_instance(3).expect("Instance is initialized. qed");
		time_b.update_time();
		let time_b = CurrTimeProvider::get_instance(4).expect("Instance is initialized. qed");
		assert_eq!(
			time_a.current_time().as_duration().as_secs() as u64,
			START_DATE + delta_a.as_secs() as u64
		);
		assert_eq!(
			time_b.current_time().as_duration().as_secs() as u64,
			START_DATE + delta_b.as_secs() as u64
		);

		// Progress time by delta
		time_a.update_time();
		let time_a = CurrTimeProvider::get_instance(3).expect("Instance is initialized. qed");
		time_b.update_time();
		let time_b = CurrTimeProvider::get_instance(4).expect("Instance is initialized. qed");
		assert_eq!(
			time_a.current_time().as_duration().as_secs() as u64,
			START_DATE + 2 * delta_a.as_secs() as u64
		);
		assert_eq!(
			time_b.current_time().as_duration().as_secs() as u64,
			START_DATE + 2 * delta_b.as_secs() as u64
		);

		let time_a_2 = CurrTimeProvider::get_instance(3).expect("Instance is available. qed");
		let time_b_2 = CurrTimeProvider::get_instance(4).expect("Instance is available. qed");

		assert_eq!(time_a.current_time(), time_a_2.current_time());
		assert_eq!(time_b.current_time(), time_b_2.current_time());
	}
}
