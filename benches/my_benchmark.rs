use anyhow::Result;
use async_utf8_decoder::Utf8Decoder;
use futures::channel::mpsc;
use futures::io;
use futures::prelude::*;
use futures::try_join;

use criterion::async_executor::FuturesExecutor;
use criterion::{black_box, criterion_group, criterion_main, Criterion};

async fn decode_hearts_one_by_one(n: usize, hearts: &[u8]) -> Result<()> {
    let (mut tx, rx) = mpsc::unbounded::<io::Result<Vec<u8>>>();
    let mut decoder = Utf8Decoder::new(rx.into_async_read());

    let producer = async {
        for _ in 1..n {
            for b in hearts {
                tx.send(Ok(vec![*b])).await?;
            }
        }
        drop(tx);
        Ok(()) as Result<()>
    };
    let consumer = async {
        while let Some(Ok(_)) = decoder.next().await {
            // Do NOTHING
        }
        Ok(()) as Result<()>
    };

    try_join!(producer, consumer)?;

    Ok(())
}

fn criterion_benchmark(c: &mut Criterion) {
    let hearts = include_bytes!("./hearts.txt");
    c.bench_function("decode x10", |b| {
        b.to_async(FuturesExecutor)
            .iter(|| decode_hearts_one_by_one(black_box(10), hearts))
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
