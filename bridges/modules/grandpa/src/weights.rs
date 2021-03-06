// Copyright 2019-2021 Parity Technologies (UK) Ltd.
// This file is part of Parity Bridges Common.

// Parity Bridges Common is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Parity Bridges Common is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Parity Bridges Common.  If not, see <http://www.gnu.org/licenses/>.

//! Autogenerated weights for pallet_bridge_grandpa
//!
//! THIS FILE WAS AUTO-GENERATED USING THE SUBSTRATE BENCHMARK CLI VERSION 3.0.0
//! DATE: 2021-04-21, STEPS: [50, ], REPEAT: 20
//! LOW RANGE: [], HIGH RANGE: []
//! EXECUTION: Some(Wasm), WASM-EXECUTION: Compiled
//! CHAIN: Some("dev"), DB CACHE: 128

// Executed Command:
// target/release/rialto-bridge-node
// benchmark
// --chain=dev
// --steps=50
// --repeat=20
// --pallet=pallet_bridge_grandpa
// --extrinsic=*
// --execution=wasm
// --wasm-execution=Compiled
// --heap-pages=4096
// --output=./modules/grandpa/src/weights.rs
// --template=./.maintain/rialto-weight-template.hbs

#![allow(clippy::all)]
#![allow(unused_parens)]
#![allow(unused_imports)]

use frame_support::{
	traits::Get,
	weights::{constants::RocksDbWeight, Weight},
};
use sp_std::marker::PhantomData;

/// Weight functions needed for pallet_bridge_grandpa.
pub trait WeightInfo {
	fn submit_finality_proof(v: u32, p: u32) -> Weight;
	fn submit_finality_proof_on_single_fork(v: u32) -> Weight;
	fn submit_finality_proof_on_many_forks(p: u32) -> Weight;
	fn find_scheduled_change(n: u32) -> Weight;
	fn read_write_authority_sets(n: u32) -> Weight;
}

/// Weights for pallet_bridge_grandpa using the Rialto node and recommended hardware.
pub struct RialtoWeight<T>(PhantomData<T>);
impl<T: frame_system::Config> WeightInfo for RialtoWeight<T> {
	fn submit_finality_proof(v: u32, p: u32) -> Weight {
		(0 as Weight)
			.saturating_add((756_462_000 as Weight).saturating_mul(v as Weight))
			.saturating_add((791_236_000 as Weight).saturating_mul(p as Weight))
			.saturating_add(T::DbWeight::get().reads(7 as Weight))
			.saturating_add(T::DbWeight::get().writes(6 as Weight))
	}
	fn submit_finality_proof_on_single_fork(v: u32) -> Weight {
		(280_121_000 as Weight)
			.saturating_add((14_098_000 as Weight).saturating_mul(v as Weight))
			.saturating_add(T::DbWeight::get().reads(7 as Weight))
			.saturating_add(T::DbWeight::get().writes(6 as Weight))
	}
	fn submit_finality_proof_on_many_forks(p: u32) -> Weight {
		(10_370_940_000 as Weight)
			.saturating_add((96_902_000 as Weight).saturating_mul(p as Weight))
			.saturating_add(T::DbWeight::get().reads(7 as Weight))
			.saturating_add(T::DbWeight::get().writes(6 as Weight))
	}
	fn find_scheduled_change(n: u32) -> Weight {
		(479_000 as Weight).saturating_add((11_000 as Weight).saturating_mul(n as Weight))
	}
	fn read_write_authority_sets(n: u32) -> Weight {
		(8_030_000 as Weight)
			.saturating_add((232_000 as Weight).saturating_mul(n as Weight))
			.saturating_add(T::DbWeight::get().reads(1 as Weight))
			.saturating_add(T::DbWeight::get().writes(1 as Weight))
	}
}

// For backwards compatibility and tests
impl WeightInfo for () {
	fn submit_finality_proof(v: u32, p: u32) -> Weight {
		(0 as Weight)
			.saturating_add((756_462_000 as Weight).saturating_mul(v as Weight))
			.saturating_add((791_236_000 as Weight).saturating_mul(p as Weight))
			.saturating_add(RocksDbWeight::get().reads(7 as Weight))
			.saturating_add(RocksDbWeight::get().writes(6 as Weight))
	}
	fn submit_finality_proof_on_single_fork(v: u32) -> Weight {
		(280_121_000 as Weight)
			.saturating_add((14_098_000 as Weight).saturating_mul(v as Weight))
			.saturating_add(RocksDbWeight::get().reads(7 as Weight))
			.saturating_add(RocksDbWeight::get().writes(6 as Weight))
	}
	fn submit_finality_proof_on_many_forks(p: u32) -> Weight {
		(10_370_940_000 as Weight)
			.saturating_add((96_902_000 as Weight).saturating_mul(p as Weight))
			.saturating_add(RocksDbWeight::get().reads(7 as Weight))
			.saturating_add(RocksDbWeight::get().writes(6 as Weight))
	}
	fn find_scheduled_change(n: u32) -> Weight {
		(479_000 as Weight).saturating_add((11_000 as Weight).saturating_mul(n as Weight))
	}
	fn read_write_authority_sets(n: u32) -> Weight {
		(8_030_000 as Weight)
			.saturating_add((232_000 as Weight).saturating_mul(n as Weight))
			.saturating_add(RocksDbWeight::get().reads(1 as Weight))
			.saturating_add(RocksDbWeight::get().writes(1 as Weight))
	}
}
