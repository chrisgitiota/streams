// Rust
use std::convert::TryFrom;

// 3rd-party
use anyhow::Result;
use chrono::Utc;
use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use serde::Deserialize;

// IOTA
use iota_sdk::{
    types::block::{
        Block,
    }
};

// Streams
use lets::{
    address::{Address, AppAddr, MsgId},
    id::Identifier,
    message::{Message, Topic, TransportMessage},
    transport::{tangle, utangle, Transport},
};

// TODO: A MessageIndex implementation is needed to create a tabgle client for the stardust IOTA
//       mainnet. For test purposes this can be achieved using a private tangle running Hornet nodes
//       and a modified inx-collector (streams-collector). For production purposes a dedicated VPS
//       will be needed to run a Hornet node and inx-plugins.

const DEFAULT_NODE: &str = "https://chrysalis-nodes.iota.org";

async fn send_message<T>(client: &mut T, payload_size: usize) -> Result<()>
where
    T: for<'a> Transport<'a, Msg = TransportMessage, SendResponse = Ignore>,
{
    let msg = TransportMessage::new(vec![12u8; payload_size]);
    let address = Address::new(
        AppAddr::default(),
        MsgId::gen(
            AppAddr::default(),
            &Identifier::default(),
            &Topic::default(),
            Utc::now().timestamp_millis() as u32,
        ),
    );
    client.send_message(address, msg).await?;
    Ok(())
}

fn bench_clients(c: &mut Criterion) {
    let url = std::env::var("NODE_URL").unwrap_or_else(|_| String::from(DEFAULT_NODE));
    let mut group = c.benchmark_group("Send Message by Size");
    let runtime = tokio::runtime::Runtime::new().unwrap();
    for i in [32, 64, 128, 256, 512, 1024] {
        group.throughput(Throughput::Bytes(i as u64));
        group.bench_with_input(BenchmarkId::new("iota.rs", i.clone()), &i, |b, payload_size| {
            b.iter_batched(
                || runtime.block_on(tangle::Client::for_node(&url)).unwrap(),
                |mut client| {
                    runtime.block_on(async {
                        send_message(&mut client, payload_size.clone()).await.unwrap();
                    })
                },
                criterion::BatchSize::SmallInput,
            )
        });
        group.bench_with_input(BenchmarkId::new("uTangle", i.clone()), &i, |b, payload_size| {
            b.iter_batched(
                || utangle::Client::new(&url),
                |mut client| {
                    runtime.block_on(async {
                        send_message(&mut client, payload_size.clone()).await.unwrap();
                    })
                },
                criterion::BatchSize::SmallInput,
            )
        });
    }
    group.finish();
}

#[derive(Deserialize)]
struct Ignore {}

impl<Payload> TryFrom<Message<Payload>> for Ignore {
    type Error = create::error::Error;
    fn try_from(_: Message<Payload>) -> Result<Self, Self::Error> {
        Ok(Ignore {})
    }
}

criterion_group!(benches, bench_clients);
criterion_main!(benches);
