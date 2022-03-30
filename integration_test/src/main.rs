fn main() {}

#[fudge::companion]
struct TestEnv {
	#[fudge::parachain(2001)]
	centrifuge: (),
	#[fudge::parachain(2002)]
	acala: (),
	#[fudge::relaychain]
	polkadot: (),
}
