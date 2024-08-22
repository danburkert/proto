use core::error::Error;
use criterion::{criterion_group, criterion_main, Criterion};
use prost::Message;

pub mod benchmarks {
    include!(concat!(env!("OUT_DIR"), "/benchmarks.rs"));

    pub mod dataset {
        pub fn google_message1_proto2() -> &'static [u8] {
            include_bytes!("../../third_party/old_protobuf_benchmarks/datasets/google_message1/proto2/dataset.google_message1_proto2.pb")
        }

        pub fn google_message1_proto3() -> &'static [u8] {
            include_bytes!("../../third_party/old_protobuf_benchmarks/datasets/google_message1/proto3/dataset.google_message1_proto3.pb")
        }

        pub fn google_message2() -> &'static [u8] {
            include_bytes!("../../third_party/old_protobuf_benchmarks/datasets/google_message2/dataset.google_message2.pb")
        }
    }

    pub mod proto2 {
        include!(concat!(env!("OUT_DIR"), "/benchmarks.proto2.rs"));
    }
    pub mod proto3 {
        include!(concat!(env!("OUT_DIR"), "/benchmarks.proto3.rs"));
    }
}

use crate::benchmarks::BenchmarkDataset;

fn load_dataset(dataset: &[u8]) -> Result<BenchmarkDataset, Box<dyn Error>> {
    Ok(BenchmarkDataset::decode(dataset)?)
}

fn benchmark_dataset<M>(criterion: &mut Criterion, name: &str, dataset: &'static [u8])
where
    M: prost::Message + Default + 'static,
{
    let mut group = criterion.benchmark_group(&format!("dataset/{}", name));

    group.bench_function("merge", move |b| {
        let dataset = load_dataset(dataset).unwrap();
        let mut message = M::default();
        b.iter(|| {
            for buf in &dataset.payload {
                message.clear();
                message.merge(buf.as_slice()).unwrap();
                criterion::black_box(&message);
            }
        });
    });

    group.bench_function("encode", move |b| {
        let messages = load_dataset(dataset)
            .unwrap()
            .payload
            .iter()
            .map(Vec::as_slice)
            .map(M::decode)
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        let mut buf = Vec::with_capacity(messages.iter().map(M::encoded_len).sum::<usize>());
        b.iter(|| {
            buf.clear();
            for message in &messages {
                message.encode(&mut buf).unwrap();
            }
            criterion::black_box(&buf);
        });
    });

    group.bench_function("encoded_len", move |b| {
        let messages = load_dataset(dataset)
            .unwrap()
            .payload
            .iter()
            .map(Vec::as_slice)
            .map(M::decode)
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        b.iter(|| {
            let encoded_len = messages.iter().map(M::encoded_len).sum::<usize>();
            criterion::black_box(encoded_len)
        });
    });
}

macro_rules! dataset {
    ($name: ident, $ty: ty) => {
        fn $name(criterion: &mut Criterion) {
            benchmark_dataset::<$ty>(
                criterion,
                stringify!($name),
                crate::benchmarks::dataset::$name(),
            );
        }
    };
}

dataset!(
    google_message1_proto2,
    crate::benchmarks::proto2::GoogleMessage1
);
dataset!(
    google_message1_proto3,
    crate::benchmarks::proto3::GoogleMessage1
);
dataset!(google_message2, crate::benchmarks::proto2::GoogleMessage2);

criterion_group!(
    dataset,
    google_message1_proto2,
    google_message1_proto3,
    google_message2,
);

criterion_main!(dataset);
