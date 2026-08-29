use std::net::SocketAddr;
use std::sync::Arc;

use tauri::{AppHandle, Emitter};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::oneshot;

use crate::core::remote::state::{gateway_status, remote_state, RemoteGatewayState};

/// Starts the WebSocket remote gateway on the configured port.
pub fn start_gateway(app: AppHandle, port: Option<u16>) -> Result<(), String> {
    let state = remote_state(&app);
    if state.is_running() {
        state.set_enabled_preference(true, port.or(Some(state.port())));
        return Ok(());
    }

    let port = port.unwrap_or_else(|| state.preferred_port());
    state.set_enabled_preference(true, Some(port));
    let (stop_tx, stop_rx) = oneshot::channel::<()>();
    state.set_stop_sender(stop_tx);

    let app_for_task = app.clone();
    let state_for_task = state.clone();

    tauri::async_runtime::spawn(async move {
        if let Err(error) = run_server(app_for_task.clone(), state_for_task, port, stop_rx).await {
            tracing::warn!(error = %error, "remote gateway stopped with error");
        }
        let state = remote_state(&app_for_task);
        state.set_running(false, port);
        let _ = app_for_task.emit("remote-gateway-status", gateway_status(&app_for_task));
    });

    Ok(())
}

async fn run_server(
    app: AppHandle,
    state: Arc<RemoteGatewayState>,
    port: u16,
    mut stop_rx: oneshot::Receiver<()>,
) -> Result<(), String> {
    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    let listener = TcpListener::bind(addr)
        .await
        .map_err(|e| format!("bind {addr} failed: {e}"))?;
    state.set_running(true, port);
    let _ = app.emit("remote-gateway-status", gateway_status(&app));
    tracing::info!(%addr, "remote gateway listening");

    loop {
        tokio::select! {
            _ = &mut stop_rx => {
                tracing::info!("remote gateway stop requested");
                break;
            }
            accepted = listener.accept() => {
                match accepted {
                    Ok((stream, peer)) => {
                        let app = app.clone();
                        let state = state.clone();
                        tauri::async_runtime::spawn(async move {
                            if let Err(error) = handle_client(app, state, stream, peer).await {
                                tracing::debug!(%peer, error = %error, "remote client closed");
                            }
                        });
                    }
                    Err(error) => {
                        tracing::warn!(error = %error, "remote gateway accept failed");
                    }
                }
            }
        }
    }
    Ok(())
}

async fn handle_client(
    app: AppHandle,
    state: Arc<RemoteGatewayState>,
    stream: TcpStream,
    peer: SocketAddr,
) -> Result<(), String> {
    let _ = stream.set_nodelay(true);
    crate::core::remote::http_proxy::dispatch(app, state, stream, peer).await
}
