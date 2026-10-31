use criterion::{criterion_group, criterion_main, Criterion};
use s_expr_core::{parse, pretty_print};

fn sample_zerotree_like_query() -> &'static str {
    "(program
        (stage
            (scan :node-type \"User\")
            (where (= :country \"US\"))
            (return :id :name))
        (stage
            (scan :node-type \"Order\")
            (where (> :total 100))
            (return :id :user-id :total)))"
}

fn sample_lvs_like_query() -> &'static str {
    "(program
        (stage
            (scan :node-type \"GeoCell\")
            (where (= :resolution 9))
            (return :id :h3-index))
        (stage
            (scan :node-type \"Event\")
            (where (= :event-type \"click\"))
            (return :id :geo-cell-id)))"
}

fn deep_nested_list(depth: usize) -> String {
    let mut s = String::new();
    for _ in 0..depth {
        s.push('(');
        s.push('x');
        s.push(' ');
    }
    s.push('x');
    for _ in 0..depth {
        s.push(')');
    }
    s
}

fn long_flat_list(len: usize) -> String {
    let mut s = String::from("(");
    for i in 0..len {
        if i > 0 {
            s.push(' ');
        }
        s.push('x');
    }
    s.push(')');
    s
}

fn bench_parse(c: &mut Criterion) {
    let input = sample_zerotree_like_query();
    c.bench_function("parse_zerotree_like", |b| {
        b.iter(|| {
            let _ = parse(std::hint::black_box(input)).expect("parse ok");
        });
    });
}

fn bench_pretty_and_parse_roundtrip(c: &mut Criterion) {
    let input = sample_lvs_like_query();
    c.bench_function("parse_pretty_parse_roundtrip", |b| {
        b.iter(|| {
            let first = parse(std::hint::black_box(input)).expect("parse ok");
            let printed = pretty_print(&first);
            let _second = parse(&printed).expect("parse ok");
        });
    });
}

fn bench_parse_deep_nested(c: &mut Criterion) {
    // Stays within default depth limits while stressing recursion/stack behavior.
    let input = deep_nested_list(64);
    c.bench_function("parse_deep_nested_64", |b| {
        b.iter(|| {
            let _ = parse(std::hint::black_box(&input)).expect("parse ok");
        });
    });
}

fn bench_parse_long_flat(c: &mut Criterion) {
    // Stays within the default max_list_length (4096) while stressing long flat lists.
    let input = long_flat_list(4000);
    c.bench_function("parse_long_flat_4000", |b| {
        b.iter(|| {
            let _ = parse(std::hint::black_box(&input)).expect("parse ok");
        });
    });
}

criterion_group!(
    benches,
    bench_parse,
    bench_pretty_and_parse_roundtrip,
    bench_parse_deep_nested,
    bench_parse_long_flat
);
criterion_main!(benches);
