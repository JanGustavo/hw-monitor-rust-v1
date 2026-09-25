//! Servidor web opcional: HTML + WebSocket em http://127.0.0.1:7878.
//! Usa o mesmo coletor de collector.rs — sem duplicação de lógica.

use axum::{
    extract::{ws::{Message, WebSocket, WebSocketUpgrade}, State},
    response::IntoResponse,
    routing::get,
    Router,
};
use std::sync::Arc;

// Importa o coletor compartilhado via o crate raiz.
use hw_monitor::collector;

async fn index() -> impl IntoResponse {
    axum::response::Html(include_str!("../../static/index.html"))
}

async fn ws(
    ws: WebSocketUpgrade,
    State(tx): State<Arc<tokio::sync::broadcast::Sender<String>>>,
) -> impl IntoResponse {
    ws.on_upgrade(move |mut socket: WebSocket| async move {
        let mut rx = tx.subscribe();
        while let Ok(msg) = rx.recv().await {
            if socket.send(Message::Text(msg)).await.is_err() { break; }
        }
    })
}

#[tokio::main]
async fn main() {
    // Inicia o coletor e expõe seu canal de broadcast.
    let (tx, _rx) = tokio::sync::broadcast::channel::<String>(16);
    let tx = Arc::new(tx);

    // Roda o loop de coleta em background.
    {
        let tx2 = Arc::clone(&tx);
        // collector::start_broadcast inicia a tarefa e envia JSON pelo Sender.
        collector::start_broadcast(tx2);
    }

    let app = Router::new()
        .route("/", get(index))
        .route("/ws", get(ws))
        .with_state(Arc::clone(&tx));

    println!("http://127.0.0.1:7878");
    axum::Server::bind(&"127.0.0.1:7878".parse().unwrap())
        .serve(app.into_make_service())
        .await
        .unwrap();
}
