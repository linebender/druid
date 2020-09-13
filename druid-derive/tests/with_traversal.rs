use druid::{Lens, Prism};

#[derive(Debug, Lens, PartialEq, Clone, Default)]
struct Person {
    pub name: String,
    pub addr: Addr,
}

#[derive(Debug, Prism, PartialEq, Clone)]
pub enum Addr {
    Long(String),
    Short(String),
    Extensive(ExtensiveAddr),
}

#[derive(Debug, Lens, PartialEq, Clone, Default)]
pub struct ExtensiveAddr {
    pub street: String,
    pub place: Place,
}

#[derive(Debug, Prism, PartialEq, Clone)]
pub enum Place {
    House(House),
    Apartment(Apartment),
}

#[derive(Debug, Lens, PartialEq, Clone, Default)]
pub struct House {
    house_number: u64,
}

#[derive(Debug, Lens, PartialEq, Clone, Default)]
pub struct Apartment {
    building_number: u64,
    floor_number: i8,
    apartment_number: u32,
}

#[test]
fn derive_traversal() {
    use druid::PrismExt;

    let mut alice = Person {
        name: "Alice".into(),
        addr: Addr::Extensive(ExtensiveAddr {
            street: "alice's street".into(),
            place: Place::House(House { house_number: 2 }),
        }),
    };

    let mut bob = Person {
        name: "Bob".into(),
        addr: Addr::Extensive(ExtensiveAddr {
            street: "bob's street".into(),
            place: Place::Apartment(Apartment {
                building_number: 3,
                floor_number: -1,
                apartment_number: 4,
            }),
        }),
    };

    let mut carl = Person {
        name: "Carl".into(),
        addr: Addr::Short("carl's short address".into()),
    };

    // all basic lenses and prisms
    let _person_name = Person::name; // lens
    let person_addr = Person::addr; // lens
    let _addr_long = Addr::long; // prism
    let _addr_short = Addr::short; // prism
    let addr_extensive = Addr::extensive; // prism
    let _ext_street = ExtensiveAddr::street; // lens
    let ext_place = ExtensiveAddr::place; // lens
    let _place_house = Place::house; // prism
    let place_apt = Place::apartment; // prism
    let _house_number = House::house_number; // lens
    let _apt_building = Apartment::building_number; // lens
    let _apt_floor = Apartment::floor_number; // lens
    let apt_number = Apartment::apartment_number; // lens

    // traversal for A -> B -> C -> D -> E -> F
    let _person_apt_number = {
        use druid::traversal::ThenAffineTraversal;
        (Person::addr) // A -> B
            .then(Addr::extensive) // B -> C
            .then(ExtensiveAddr::place) // C -> D
            .then(Place::apartment) // D -> E
            .then(Apartment::apartment_number) // E -> F
    };

    // traversal for A -> B -> C -> D -> E -> F
    let person_apt_number = {
        use druid::traversal::ThenAffineTraversal;
        (Person::addr) // A -> B
            .then(Addr::extensive) // B -> C
            .then(ExtensiveAddr::place) // C -> D
            .then(Place::apartment) // D -> E
            .then(Apartment::apartment_number) // E -> F
    };

    // alternative traversal for A -> B -> C -> D -> E -> F
    // (the outer type ends up different)
    let person_apt_number_2 = {
        use druid::traversal::ThenAffineTraversal;
        let place_apt_number = place_apt.then(apt_number); // D -> E -> F
        let addr_ext_place = addr_extensive.then(ext_place); // B -> C -> D

        // B -> C -> D -> E -> F
        let addr_apt_number = addr_ext_place.then(place_apt_number);
        // A -> B -> C -> D -> E -> F
        person_addr.then(addr_apt_number)
    };

    // gets the apartment number (only bob has)
    assert_eq!(None, person_apt_number.with(&alice, |_n| unreachable!()));
    assert_eq!(Some(4), person_apt_number.get(&bob));
    assert_eq!(None, person_apt_number.with(&carl, |_n| unreachable!()));

    // again, gets the apartment number (only bob has)
    assert_eq!(None, person_apt_number_2.with(&alice, |_n| unreachable!()));
    assert_eq!(Some(4), person_apt_number_2.get(&bob));
    assert_eq!(None, person_apt_number_2.with(&carl, |_n| unreachable!()));

    // changes the apartment number (only bob has)
    assert_eq!(
        None,
        person_apt_number.with_mut(&mut alice, |_n| unreachable!())
    );
    assert_eq!(Some(()), person_apt_number.with_mut(&mut bob, |n| *n = 7));
    assert_eq!(
        None,
        person_apt_number.with_mut(&mut carl, |_n| unreachable!())
    );

    // confirms bob changed the apartment number (from 4 to 7)
    assert_eq!(Some(7), person_apt_number.get(&bob));

    // traversal for A -> B -> C -> D
    let person_place = {
        use druid::traversal::ThenAffineTraversal;
        (Person::addr) // A -> B
            .then(Addr::extensive) // B -> C
            .then(ExtensiveAddr::place) // C -> D
    };

    // alice will change from a House into a new Apartment
    {
        use druid::optics::Replace;

        // required for replace/upgrade (which is forceful)
        impl Default for Place {
            fn default() -> Self {
                Self::House(House { house_number: 0 })
            }
        }

        // alice will change from a House into a new Apartment
        assert_eq!(
            Some(()),
            person_place.with_mut(&mut alice, |alice_place| {
                place_apt.replace(
                    alice_place,
                    Apartment {
                        building_number: 2,
                        floor_number: 1,
                        apartment_number: 100,
                    },
                );
            })
        );

        // confirms alice's new apartment number
        assert_eq!(Some(100), person_apt_number.get(&alice));
    }

    // carl will move into bob's place
    {
        use druid::optics::Replace;

        // required for replace/upgrade (which is forceful)
        impl Default for Addr {
            fn default() -> Self {
                Self::Short("".into())
            }
        }

        // carl will move into bob's place
        assert_eq!(
            Some(()),
            person_place.with(&bob, |bob_place| {
                person_place.replace(&mut carl, bob_place.clone());
            })
        );

        // confirm carl's new place
        assert_eq!(person_place.get(&bob), person_place.get(&carl));
        // confirms carl's new apartment number
        assert_eq!(Some(7), person_apt_number.get(&carl));

        // note that the street name, which is outside of the
        // Place enum, is still old (aka wrong).
        // for it to be correct, the entire ExtensiveAddr
        // should have been replaced.
    }
}
