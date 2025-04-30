use tokio::{
    net::TcpListener,
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    sync::broadcast,
};
use serde::Deserialize;
use std::fs;
use clap::Parser;

#[derive(Parser, Debug)]
struct Args {
    #[arg(short, long, default_value = "config.toml")]
    config: String,
}

#[derive(Deserialize, Debug)]
struct Config {
    central_ip_port: u32,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let config = load_config(&args.config)?;

    println!("config loaded!");

    let listener = TcpListener::bind(format!("0.0.0.0:{}", config.central_ip_port.to_string())).await?;
    println!("Central process started on port 4000");

    let (tx, _) = broadcast::channel::<String>(100);

    loop {
        let (stream, addr) = listener.accept().await?;
        println!("Client connected: {}", addr);

        let tx = tx.clone();
        let mut rx = tx.subscribe();

        tokio::spawn(async move {
            let (reader, mut writer) = stream.into_split();
            let reader = BufReader::new(reader);
            let mut lines = reader.lines();

            let read_task = tokio::spawn({
                let tx = tx.clone();
                async move {
                    while let Ok(Some(line)) = lines.next_line().await {
                        println!("Received from {}: {}", addr, line);
                        if let Err(e) = tx.send(line) {
                            eprintln!("Broadcast error: {}", e);
                        }
                    }
                }
            });

            let write_task = tokio::spawn(async move {
                while let Ok(msg) = rx.recv().await {
                    writer.write_all(msg.as_bytes()).await.ok();
                    writer.write_all(b"\n").await.ok();
                }
            });

            let _ = tokio::try_join!(read_task, write_task);
            println!("Client disconnected: {}", addr);
        });
    }
}

fn load_config(path: &str) -> anyhow::Result<Config> {
    let text = fs::read_to_string(path)?;
    let config: Config = toml::from_str(&text)?;
    Ok(config)
}
