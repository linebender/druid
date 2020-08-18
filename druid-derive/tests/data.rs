//! Test #[derive(Data)]

use druid::Data;

#[derive(Data, Clone)]
struct PlainStruct;

#[derive(Data, Clone)]
struct EmptyTupleStruct();

#[derive(Data, Clone)]

struct SingleTupleStruct(bool);

#[derive(Data, Clone)]
struct MultiTupleStruct(bool, i64, String);

#[derive(Data, Clone)]
struct EmptyFieldStruct {}

#[derive(Data, Clone)]
struct SingleFieldStruct {
    a: bool,
}

#[derive(Data, Clone)]
struct MultiFieldStruct {
    a: bool,
    b: i64,
    c: String,
}

trait UserTrait {}

#[derive(Clone, Data)]
struct TypeParamForUserTraitStruct<T: UserTrait + Data> {
    a: T,
}

#[derive(Clone, Data)]
struct TypeParamForUserTraitWithWhereClauseStruct<T>
where
    T: UserTrait,
{
    b: T,
}

#[derive(Clone, Data)]
enum TypeParamForUserTraitAndLifetimeEnum<T: UserTrait + 'static> {
    V1(T),
}

#[test]
fn test_struct_data_derive_same() {
    let plain = PlainStruct;
    assert!(plain.same(&plain));

    let empty_tuple = EmptyTupleStruct();
    assert!(empty_tuple.same(&empty_tuple));

    let singletuple = SingleTupleStruct(true);
    assert!(singletuple.same(&singletuple));
    assert_eq!(false, singletuple.same(&SingleTupleStruct(false)));

    let multituple = MultiTupleStruct(false, 33, "Test".to_string());
    assert!(multituple.same(&multituple));
    assert_eq!(
        false,
        multituple.same(&MultiTupleStruct(true, 33, "Test".to_string()))
    );

    let empty_field = EmptyFieldStruct {};
    assert!(empty_field.same(&empty_field));

    let singlefield = SingleFieldStruct { a: true };
    assert!(singlefield.same(&singlefield));
    assert_eq!(false, singlefield.same(&SingleFieldStruct { a: false }));

    let multifield = MultiFieldStruct {
        a: false,
        b: 33,
        c: "Test".to_string(),
    };
    assert!(multifield.same(&multifield));
    assert_eq!(
        false,
        multifield.same(&MultiFieldStruct {
            a: false,
            b: 33,
            c: "Fail".to_string()
        })
    );

    #[derive(Clone, Data)]
    struct Value(u32);

    impl UserTrait for Value {}

    let v = TypeParamForUserTraitStruct { a: Value(1) };
    assert!(v.same(&v));
    assert_eq!(false, v.same(&TypeParamForUserTraitStruct { a: Value(2) }));

    let v = TypeParamForUserTraitWithWhereClauseStruct { b: Value(3) };
    assert!(v.same(&v));
    assert_eq!(
        false,
        v.same(&TypeParamForUserTraitWithWhereClauseStruct { b: Value(6) })
    );

    let v = TypeParamForUserTraitAndLifetimeEnum::V1(Value(10));
    assert!(v.same(&v));
    assert_eq!(
        false,
        v.same(&TypeParamForUserTraitAndLifetimeEnum::V1(Value(12)))
    );
}

// cannot really test it
#[derive(Data, Clone, PartialEq)]
enum EmptyEnum {}

#[derive(Data, Clone, PartialEq)]

enum SingleNameVariantEnum {
    Variant,
}

#[derive(Data, Clone, PartialEq)]
enum MultiNameVariantEnum {
    A,
    Bb,
    Ccc,
}

#[derive(Data, Clone, PartialEq)]
enum SingleTupleVariantEnum {
    Variant(bool),
}

#[derive(Data, Clone, PartialEq)]
enum SingleMultiTupleVariantEnum {
    Variant(bool, u8),
}

#[derive(Data, Clone, PartialEq)]
enum MultiTupleVariantEnum {
    A(bool),
    Bb(u8),
}

#[derive(Data, Clone, PartialEq)]
enum MultiMultiTupleVariantEnum {
    A(bool, u8),
    Bb(u8, u16),
}

#[derive(Data, Clone, PartialEq)]
enum SingleStructVariantEnum {
    Variant { x: bool },
}

#[derive(Data, Clone, PartialEq)]
enum SingleMultiStructVariantEnum {
    Variant { x: bool, y: u8 },
}

#[derive(Data, Clone, PartialEq)]
enum MultiStructVariantEnum {
    A { x: bool },
    Bb { x: u8 },
}

#[derive(Data, Clone, PartialEq)]
enum MultiMultiStructVariantEnum {
    A { x: bool, y: u8 },
    Bb { a: u8, b: u16 },
}

#[derive(Data, Clone, PartialEq)]
enum MultiMixedVariantEnum {
    Named,
    SingleTuple(bool),
    MultiTuple(u8, u16),
    SingleStruct { x: bool },
    MultiStruct { x: u8, y: u16 },
}

#[test]
fn test_enum_data_derive_same() {
    // empty enum is skipped as it cannot be tested

    let single_name = SingleNameVariantEnum::Variant;
    assert!(single_name.same(&single_name));

    let multi_name = MultiNameVariantEnum::A;
    assert!(multi_name.same(&multi_name));
    assert_eq!(false, multi_name.same(&MultiNameVariantEnum::Bb));
    assert_eq!(false, multi_name.same(&MultiNameVariantEnum::Ccc));

    let single_tuple = SingleTupleVariantEnum::Variant(true);
    assert!(single_tuple.same(&single_tuple));
    assert_eq!(
        false,
        single_tuple.same(&SingleTupleVariantEnum::Variant(false))
    );

    let single_multi_tuple = SingleMultiTupleVariantEnum::Variant(true, 1);
    assert!(single_multi_tuple.same(&single_multi_tuple));
    assert_eq!(
        false,
        single_multi_tuple.same(&SingleMultiTupleVariantEnum::Variant(false, 1))
    );
    assert_eq!(
        false,
        single_multi_tuple.same(&SingleMultiTupleVariantEnum::Variant(true, 2))
    );
    assert_eq!(
        false,
        single_multi_tuple.same(&SingleMultiTupleVariantEnum::Variant(false, 2))
    );

    let multi_tuple = MultiTupleVariantEnum::A(true);
    assert!(multi_tuple.same(&multi_tuple));
    assert_eq!(false, multi_tuple.same(&MultiTupleVariantEnum::A(false)));
    assert_eq!(false, multi_tuple.same(&MultiTupleVariantEnum::Bb(0)));

    let multi_multi_tuple = MultiMultiTupleVariantEnum::A(true, 1);
    assert!(multi_multi_tuple.same(&multi_multi_tuple));
    assert_eq!(
        false,
        multi_multi_tuple.same(&MultiMultiTupleVariantEnum::A(false, 1))
    );
    assert_eq!(
        false,
        multi_multi_tuple.same(&MultiMultiTupleVariantEnum::A(true, 0))
    );
    assert_eq!(
        false,
        multi_multi_tuple.same(&MultiMultiTupleVariantEnum::A(false, 0))
    );
    assert_eq!(
        false,
        multi_multi_tuple.same(&MultiMultiTupleVariantEnum::Bb(1, 2))
    );

    let single_struct = SingleStructVariantEnum::Variant { x: true };
    assert!(single_struct.same(&single_struct));
    assert_eq!(
        false,
        single_struct.same(&SingleStructVariantEnum::Variant { x: false })
    );

    let single_multi_struct = SingleMultiStructVariantEnum::Variant { x: true, y: 1 };
    assert!(single_multi_struct.same(&single_multi_struct));
    assert_eq!(
        false,
        single_multi_struct.same(&SingleMultiStructVariantEnum::Variant { x: false, y: 1 })
    );
    assert_eq!(
        false,
        single_multi_struct.same(&SingleMultiStructVariantEnum::Variant { x: true, y: 0 })
    );
    assert_eq!(
        false,
        single_multi_struct.same(&SingleMultiStructVariantEnum::Variant { x: false, y: 0 })
    );

    let multi_struct = MultiStructVariantEnum::A { x: true };
    assert!(multi_struct.same(&multi_struct));
    assert_eq!(
        false,
        multi_struct.same(&MultiStructVariantEnum::A { x: false })
    );
    assert_eq!(
        false,
        multi_struct.same(&MultiStructVariantEnum::Bb { x: 3 })
    );

    let multi_multi_struct = MultiMultiStructVariantEnum::A { x: true, y: 1 };
    assert!(multi_multi_struct.same(&multi_multi_struct));
    assert_eq!(
        false,
        multi_multi_struct.same(&MultiMultiStructVariantEnum::A { x: false, y: 1 })
    );
    assert_eq!(
        false,
        multi_multi_struct.same(&MultiMultiStructVariantEnum::A { x: true, y: 3 })
    );
    assert_eq!(
        false,
        multi_multi_struct.same(&MultiMultiStructVariantEnum::Bb { a: 0, b: 0 })
    );

    let mixed_named = MultiMixedVariantEnum::Named;
    let mixed_single_tuple = MultiMixedVariantEnum::SingleTuple(true);
    let mixed_multi_tuple = MultiMixedVariantEnum::MultiTuple(3, 4);
    let mixed_single_struct = MultiMixedVariantEnum::SingleStruct { x: true };
    let mixed_multi_struct = MultiMixedVariantEnum::MultiStruct { x: 1, y: 2 };
    assert!(mixed_named.same(&mixed_named));
    assert_eq!(false, mixed_named.same(&mixed_single_tuple));
    assert_eq!(false, mixed_named.same(&mixed_multi_tuple));
    assert_eq!(false, mixed_named.same(&mixed_single_struct));
    assert_eq!(false, mixed_named.same(&mixed_multi_struct));
    assert!(mixed_single_tuple.same(&mixed_single_tuple));
    assert_eq!(
        false,
        mixed_single_tuple.same(&MultiMixedVariantEnum::SingleTuple(false))
    );
    assert_eq!(false, mixed_single_tuple.same(&mixed_multi_tuple));
    assert_eq!(false, mixed_single_tuple.same(&mixed_single_struct));
    assert_eq!(false, mixed_single_tuple.same(&mixed_multi_struct));
    assert!(mixed_multi_tuple.same(&mixed_multi_tuple));
    assert_eq!(
        false,
        mixed_multi_tuple.same(&MultiMixedVariantEnum::MultiTuple(3, 5))
    );
    assert_eq!(false, mixed_multi_tuple.same(&mixed_single_struct));
    assert_eq!(false, mixed_multi_tuple.same(&mixed_multi_struct));
    assert!(mixed_single_struct.same(&mixed_single_struct));
    assert_eq!(
        false,
        mixed_single_struct.same(&MultiMixedVariantEnum::SingleStruct { x: false })
    );
    assert_eq!(false, mixed_single_struct.same(&mixed_multi_struct));
    assert!(mixed_multi_struct.same(&mixed_multi_struct));
    assert_eq!(
        false,
        mixed_multi_struct.same(&MultiMixedVariantEnum::MultiStruct { x: 2, y: 2 })
    );
}
