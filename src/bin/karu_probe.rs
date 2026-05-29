use futures_util::{SinkExt, StreamExt};
use serde_json::json;
use std::time::Duration;
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};

#[tokio::main]
async fn main() {
    let url = std::env::args()
        .nth(1)
        .or_else(|| std::env::var("KARU_WS_URL").ok())
        .unwrap_or_else(|| "ws://localhost:8765".to_string());

    println!("conectando em {url}");

    let (stream, response) = match connect_async(&url).await {
        Ok(resultado) => resultado,
        Err(erro) => {
            eprintln!("falha ao conectar: {erro}");
            std::process::exit(1);
        }
    };

    println!("websocket conectado: HTTP {}", response.status());

    let (mut tx, mut rx) = stream.split();

    if let (Ok(username), Ok(password)) = (std::env::var("KARU_USER"), std::env::var("KARU_PASS")) {
        let login = json!({
            "type": "login",
            "username": username,
            "password": password
        });

        tx.send(Message::Text(login.to_string()))
            .await
            .expect("falha ao enviar login");
        println!("login enviado, aguardando resposta...");
    } else {
        println!("sem KARU_USER/KARU_PASS; testei somente a conexao ws");
        return;
    }

    match tokio::time::timeout(Duration::from_secs(8), rx.next()).await {
        Ok(Some(Ok(Message::Text(texto)))) => println!("recebido: {texto}"),
        Ok(Some(Ok(outro))) => println!("recebido nao-texto: {outro:?}"),
        Ok(Some(Err(erro))) => {
            eprintln!("erro lendo websocket: {erro}");
            std::process::exit(1);
        }
        Ok(None) => {
            eprintln!("servidor fechou a conexao");
            std::process::exit(1);
        }
        Err(_) => {
            eprintln!("timeout aguardando resposta do servidor");
            std::process::exit(1);
        }
    }
}
