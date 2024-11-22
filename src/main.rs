use std::{process::Stdio, sync::Arc, time::Duration};

use macella::Server;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    process::Command,
    sync::{broadcast, Mutex},
    time,
};

#[tokio::main]
async fn main() {
    let playlist = Arc::new(Mutex::new(Vec::<String>::new()));

    let (tx, _rx) = broadcast::channel(16);

    tokio::spawn(playlist_observe(playlist.clone(), tx.clone()));

    let _ = Server::new()
        .post("/start", move |_, body: String| {
            let playlist_clone = playlist.clone();
            async move {
                playlist_clone.lock().await.push(body);
                String::from("done")
            }
        })
        .ws("/beam", move |stream: tokio::net::TcpStream| {
            let tx_clone = tx.clone();
            beam(stream, tx_clone)
        })
        .bind("0.0.0.0:8000")
        .await;
}

fn frame_lengh_calc(num: u64) -> Vec<u8> {
    let mut bytes: Vec<u8> = Vec::new();

    let mut num_cache = num;

    while num_cache > 0 {
        bytes.insert(0, (num_cache & 0xFF) as u8);
        num_cache >>= 8;
    }

    if num > 65535 {
        bytes.insert(0, 127);
    } else if num >= 126 {
        bytes.insert(0, 126);
    }

    bytes
}

async fn beam(mut stream: tokio::net::TcpStream, tx: broadcast::Sender<Vec<u8>>) {
    let mut rx = tx.subscribe();
    while let Ok(audio_data) = rx.recv().await {
        let mut frame: Vec<u8> = Vec::new();
        frame.insert(0, 130);
        frame.extend(frame_lengh_calc(audio_data.len() as u64));
        frame.extend(audio_data);
        if let Err(e) = stream.write_all(&frame).await {
            eprintln!("stream write err: {}", e);
            break;
        }
    }
}

async fn playlist_observe(playlist: Arc<Mutex<Vec<String>>>, tx: broadcast::Sender<Vec<u8>>) {
    loop {
        let mut list = playlist.lock().await;

        if !list.is_empty() {
            let file_path = list.remove(0);
            trigger_play(file_path, tx.clone()).await;
        }

        time::sleep(Duration::from_millis(100)).await;
    }
}

async fn trigger_play(file_path: String, tx: broadcast::Sender<Vec<u8>>) {
    let mut child = Command::new("ffmpeg")
        .arg("-i")
        .arg(file_path)
        .arg("-b:a")
        .arg("128k")
        .arg("-f")
        .arg("mp3")
        .arg("-")
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();

    let mut stdout = child.stdout.take().expect("failed to open file.");

    let mut buffer = Vec::new();

    let _ = stdout.read_to_end(&mut buffer).await;

    let status = child.wait().await.unwrap();

    if status.success() {
        let chunk_size = 8000;
        for chunk in buffer.chunks(chunk_size) {
            let _ = tx.send(chunk.to_vec());
            time::sleep(Duration::from_millis(500)).await;
        }
    } else {
        eprintln!("error processing the file.")
    }
}

//async fn trigger_play_old(file_name: String, tx: broadcast::Sender<Vec<u8>>) {
//    let mut file = File::open(file_name).expect("failed to read file");
//    let mut file_data = Vec::new();
//    file.read_to_end(&mut file_data)
//        .expect("failed to read file");
//
//    let chunk_size = 8000;
//    for chunk in file_data.chunks(chunk_size) {
//        let _ = tx.send(chunk.to_vec());
//        time::sleep(Duration::from_millis(500)).await;
//    }
//}
