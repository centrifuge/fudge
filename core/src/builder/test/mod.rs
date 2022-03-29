use fudge_companion;
use std::marker::PhantomData;

pub struct ParachainBuilder<Block, RtApi>(PhantomData<(Block, RtApi)>);
pub struct RelaychainBuilder<RtApi>(PhantomData<RtApi>);
use crate::FudgeParaChain;

#[fudge_companion::companion]
pub struct TestEnv {
	#[fudge_companion::parachain(2001)]
	parachain_1: ParachainBuilder<(), ()>,
	#[fudge_companion::parachain(2002)]
	parachain_2: ParachainBuilder<u32, u32>,
	#[fudge_companion::relaychain]
	relay_chain: RelaychainBuilder<()>,
}
