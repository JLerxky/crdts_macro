# crdts_macro

[![crates.io](https://img.shields.io/crates/v/crdts_macro.svg)](https://crates.io/crates/crdts_macro)

## Usage

### Add the dependency

Add the [`crdts`](https://github.com/rust-crdt/rust-crdt) and `crdts_macro` dependency to `Cargo.toml`:

```toml
[dependencies]
crdts = "7.3"
crdts_macro = "7.3"
```

### Custom CRDT struct

```rust
use crdts::{GCounter, Map, Orswot};
use crdts_macro::crdt;

#[crdt(u64)]
pub struct Data {
    a: Orswot<String, String>,
    b: Map<u64, Orswot<Vec<u8>, u64>, u64>,
    c: Orswot<Vec<u8>, u64>,
    d: GCounter<u64>,
}
```

#### Use this struct

```rust
#[test]
fn test() {
    use crdts::{CmRDT, CvRDT, Dot};
    let mut data1 = Data::default();
    let mut data2 = data1.clone();
    let actor = 1;
    let counter = 1;
    let dot = Dot::new(actor, counter);
    let op1 = data1.a.add(
        format!("{actor}-{counter}"),
        data1.a.read_ctx().derive_add_ctx(actor.to_string()),
    );

    let add_ctx = data1.b.read_ctx().derive_add_ctx(actor);
    let op2 = data1
        .b
        .update(actor, add_ctx, |v, a| v.add(vec![actor as u8; 20], a));

    let op3 = data1
        .c
        .add(vec![actor as u8; 20], data1.c.read_ctx().derive_add_ctx(actor));

    let op4 = data1.d.inc(actor);
    data1.apply(DataCrdtOp {
        dot,
        a_op: Some(op1),
        b_op: Some(op2),
        c_op: Some(op3),
        d_op: Some(op4),
    });
    println!("data1: {:#?}", data1);

    let actor = 2;
    let counter = 1;
    let dot = Dot::new(actor, counter);
    let op1 = data2.a.add(
        format!("{actor}-{counter}"),
        data2.a.read_ctx().derive_add_ctx(actor.to_string()),
    );

    let add_ctx = data2.b.read_ctx().derive_add_ctx(actor);
    let op2 = data2
        .b
        .update(actor, add_ctx, |v, a| v.add(vec![actor as u8; 20], a));

    let op3 = data2
        .c
        .add(vec![actor as u8; 20], data2.c.read_ctx().derive_add_ctx(actor));

    let op4 = data2.d.inc(actor);
    data2.apply(DataCrdtOp {
        dot,
        a_op: Some(op1),
        b_op: Some(op2),
        c_op: Some(op3),
        d_op: Some(op4),
    });
    println!("data2: {:#?}", data2);

    data1.merge(data2);
    println!("data3: {:#?}", data1);
}

```

## Compatible crdts versions

Compatibility of `crdts_macro` versions:

| `crdts_macro` | `crdts` |
| :--           | :--    |
| `7.3`         | `7.3`  |