use scalia_core::fixed::Fix;
use scalia_core::groups::{GroupSpec, Groups};
use scalia_core::position::{Pos, RegionId};
use scalia_core::roster::{Roster, SoldierRec};
use scalia_core::units::{Composition, Health, Needs};

fn origin() -> Pos {
    Pos {
        region: RegionId(0),
        x: 0,
        y: 0,
        z: 0,
    }
}

fn spec(c: [u32; 4]) -> GroupSpec {
    GroupSpec {
        pos: origin(),
        composition: Composition(c),
        needs: Needs::full(),
    }
}

#[test]
fn soldier_rec_is_8_bytes() {
    assert_eq!(std::mem::size_of::<SoldierRec>(), 8);
}

#[test]
fn spawn_allocates_roster() {
    let mut roster = Roster::new();
    let mut groups = Groups::new();
    let id = groups.spawn(spec([150, 5, 0, 0]), &mut roster);
    let (start, len) = groups.roster_range(id);
    assert_eq!(len, 155);
    assert_eq!(groups.people(id), 155);
    assert_eq!(groups.len(), 1);
    for slot in 0..len {
        let r = roster.rec(start, slot);
        assert_eq!(r.health, Health::Healthy);
        assert_eq!(r.xp, 0);
    }
    assert_eq!(groups.needs(id).food, Fix::ONE);
}

#[test]
fn ranges_disjoint() {
    let mut roster = Roster::new();
    let mut groups = Groups::new();
    let a = groups.spawn(spec([100, 0, 0, 0]), &mut roster);
    let b = groups.spawn(spec([50, 5, 0, 0]), &mut roster);
    let (sa, la) = groups.roster_range(a);
    let (sb, lb) = groups.roster_range(b);
    assert!(sa + la <= sb || sb + lb <= sa);
}

#[test]
fn zero_soldier_group_allowed() {
    let mut roster = Roster::new();
    let mut groups = Groups::new();
    let id = groups.spawn(spec([0, 0, 12, 3]), &mut roster);
    assert_eq!(groups.roster_range(id).1, 0);
    assert_eq!(groups.people(id), 0);
    assert_eq!(groups.composition(id).total(), 15);
}
