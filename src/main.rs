use std::{fs::File, io::Read, time::Duration};

use macella::Server;
use tokio::{io::AsyncWriteExt, sync::broadcast, time};

#[tokio::main]
async fn main() {
    let (tx, _rx) = broadcast::channel(16);

    let tx_clone1 = tx.clone();
    let tx_clone2 = tx.clone();

    //let _ = Server::new().get("/start", &pong);
    let _ = Server::new()
        .get("/start", move |_| {
            let tx_clone = tx_clone1.clone();
            async move {
                tokio::spawn(trigger_play(tx_clone));
                String::from("done")
            }
        })
        .ws("/beam", move |stream: tokio::net::TcpStream| {
            let tx_clone = tx_clone2.clone();
            beam(stream, tx_clone)
        })
        .bind("0.0.0.0:8080")
        .await;
}

//async fn pong(_: String) -> String {
//    String::from("pong")
//}

async fn beam(mut stream: tokio::net::TcpStream, tx: broadcast::Sender<Vec<u8>>) {
    let mut rx = tx.subscribe();
    while let Ok(mut audio_data) = rx.recv().await {
        audio_data.insert(0, 64);
        audio_data.insert(0, 31);
        audio_data.insert(0, 126);
        audio_data.insert(0, 130);
        if let Err(e) = stream.write_all(&audio_data).await {
            eprintln!("stream write err: {}", e);
            break;
        }
    }
}

async fn trigger_play(tx: broadcast::Sender<Vec<u8>>) {
    let mut file = File::open("lsds.mp3").expect("failed to read file");
    let mut file_data = Vec::new();
    file.read_to_end(&mut file_data)
        .expect("failed to read file");

    let chunk_size = 8000;
    for chunk in file_data.chunks(chunk_size) {
        let _ = tx.send(chunk.to_vec());
        println!("sent");
        time::sleep(Duration::from_millis(500)).await;
    }
}
