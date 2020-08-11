use druid::Lens;
use std::fmt::Debug;
use std::marker::PhantomData;

#[derive(Lens)]
struct Wrapper<T> {
    x: T,
}

#[test]
fn one_plain_param() {
    let wrap = Wrapper::<u64> { x: 45 };
    let val = Wrapper::<u64>::x.with(&wrap, |val| *val);
    assert_eq!(wrap.x, val);

    let wrap = Wrapper::<String> { x: "pop".into() };
    let val = Wrapper::<String>::x.with(&wrap, |val| val.clone());
    assert_eq!(wrap.x, val)
}

#[derive(Lens)]
struct DebugWrapper<T: Debug> {
    x: T,
}

#[test]
fn one_trait_param() {
    let wrap = DebugWrapper::<u64> { x: 45 };
    let val = DebugWrapper::<u64>::x.with(&wrap, |val| *val);
    assert_eq!(wrap.x, val);

    let wrap = DebugWrapper::<String> { x: "pop".into() };
    let val = DebugWrapper::<String>::x.with(&wrap, |val| val.clone());
    assert_eq!(wrap.x, val)
}

#[derive(Lens)]
struct LifetimeWrapper<'a, T: 'a> {
    x: T,
    phantom_a: PhantomData<&'a T>,
}

#[test]
fn one_lifetime_param() {
    let wrap = LifetimeWrapper::<u64> {
        x: 45,
        phantom_a: Default::default(),
    };
    let val = LifetimeWrapper::<u64>::x.with(&wrap, |val| *val);
    assert_eq!(wrap.x, val);

    let wrap = LifetimeWrapper::<String> {
        x: "pop".into(),
        phantom_a: Default::default(),
    };
    let val = LifetimeWrapper::<String>::x.with(&wrap, |val| val.clone());
    assert_eq!(wrap.x, val)
}

trait XT {
    type I: YT;
}

trait YT {
    type P;
}

#[derive(Lens)]
struct WhereWrapper<T, U, W>
where
    T: XT<I = U>,
    U: YT,
{
    t: T,
    u: U,
    w: W,
}

impl XT for u64 {
    type I = i32;
}

impl YT for i32 {
    type P = bool;
}

#[test]
fn where_clause() {
    type WW = WhereWrapper<u64, i32, bool>;

    let mut wrap = WW {
        t: 45,
        u: 1_000_000,
        w: true,
    };
    let ext = (
        WW::t.with(&wrap, |val| *val),
        WW::u.with(&wrap, |val| *val),
        WW::w.with(&wrap, |val| *val),
    );

    assert_eq!((wrap.t, wrap.u, wrap.w), ext);

    WW::t.with_mut(&mut wrap, |val| *val = 67);

    assert_eq!(wrap.t, 67)
}

#[derive(Lens)]
struct ReservedParams<F, V> {
    f: F, // We were using V and F as method params
    v: V,
}

#[test]
fn reserved() {
    let rp = ReservedParams::<u64, String> {
        f: 56,
        v: "Go".into(),
    };
    let val = ReservedParams::<u64, String>::f.with(&rp, |val| *val);
    assert_eq!(rp.f, val);
}
