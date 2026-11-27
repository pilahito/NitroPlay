//! NitroPlay (Windows): login de PSN dentro de la ventana. Al llegar la redireccion de Sony
//! se corta la navegacion, se saca el `code` y se muestra el Account-ID. Todo en Rust.
#![windows_subsystem = "windows"]

use psn_account::{
    account_id_base64, basic_auth_header, code_from_redirect_url, login_url, parse_access_token,
    parse_user_id, token_body, REDIRECT_URI, TOKEN_URL,
};
use tao::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoopBuilder},
    window::WindowBuilder,
};
use wry::WebViewBuilder;

enum UiEvent {
    Code(String),
    Done(Result<String, String>),
}

fn fetch_account_id(code: &str) -> Result<String, String> {
    let auth = basic_auth_header();
    let body = token_body(code);
    let token_json = ureq::post(TOKEN_URL)
        .header("Authorization", auth.as_str())
        .header("Content-Type", "application/x-www-form-urlencoded")
        .header("Accept", "application/json")
        .send(body.as_str())
        .map_err(|e| format!("token: {e}"))?
        .body_mut()
        .read_to_string()
        .map_err(|e| format!("token (lectura): {e}"))?;
    let access = parse_access_token(&token_json).ok_or("la respuesta no trae access_token")?;
    let user_json = ureq::get(format!("{TOKEN_URL}/{access}").as_str())
        .header("Authorization", auth.as_str())
        .header("Accept", "application/json")
        .call()
        .map_err(|e| format!("usuario: {e}"))?
        .body_mut()
        .read_to_string()
        .map_err(|e| format!("usuario (lectura): {e}"))?;
    let user_id = parse_user_id(&user_json).ok_or("la respuesta no trae user_id")?;
    Ok(account_id_base64(user_id))
}

fn escape(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
}

const STYLE: &str = "body{font-family:Segoe UI,sans-serif;background:#0f1424;color:#e8ecf8;display:flex;align-items:center;justify-content:center;height:100vh;margin:0}.box{max-width:560px;text-align:center;padding:24px}h1{color:#6ea8ff}input{font-size:26px;text-align:center;width:100%;padding:12px;border-radius:8px;border:1px solid #3a4770;background:#1a2138;color:#fff}button{margin-top:14px;padding:10px 22px;font-size:16px;border:0;border-radius:8px;background:#3d7bff;color:#fff;cursor:pointer}small{color:#93a0c8}";

fn page_wait() -> String {
    format!("<html><head><meta charset='utf-8'><style>{STYLE}</style></head><body><div class='box'><h1>NitroPlay</h1><p>Sesion iniciada. Obteniendo tu Account-ID...</p></div></body></html>")
}

fn page_result(result: &Result<String, String>) -> String {
    let inner = match result {
        Ok(id) => format!(
            "<h1>Tu Account-ID</h1><input id='i' readonly value='{}' onclick='this.select()'><br><button onclick=\"var e=document.getElementById('i');e.select();document.execCommand('copy');this.textContent='Copiado'\">Copiar</button><p><small>Pegalo en el registro de la consola de NitroPlay junto al PIN de 8 digitos de la PS4/PS5.</small></p>",
            escape(id)
        ),
        Err(e) => format!("<h1>No se pudo obtener el ID</h1><p>{}</p><p><small>Cierra la ventana y vuelve a intentarlo.</small></p>", escape(e)),
    };
    format!("<html><head><meta charset='utf-8'><style>{STYLE}</style></head><body><div class='box'>{inner}</div></body></html>")
}

fn main() {
    let event_loop = EventLoopBuilder::<UiEvent>::with_user_event().build();
    let proxy = event_loop.create_proxy();
    let window = WindowBuilder::new()
        .with_title("NitroPlay - Login de PSN")
        .build(&event_loop)
        .unwrap();

    let nav_proxy = proxy.clone();
    let webview = WebViewBuilder::new()
        .with_url(login_url())
        .with_navigation_handler(move |url: String| {
            if url.starts_with(REDIRECT_URI) {
                if let Some(code) = code_from_redirect_url(&url) {
                    let _ = nav_proxy.send_event(UiEvent::Code(code));
                }
                return false; // la pagina final de Sony no hace falta cargarla
            }
            true
        })
        .build(&window)
        .unwrap();

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;
        match event {
            Event::UserEvent(UiEvent::Code(code)) => {
                let _ = webview.load_html(&page_wait());
                let p = proxy.clone();
                std::thread::spawn(move || {
                    let _ = p.send_event(UiEvent::Done(fetch_account_id(&code)));
                });
            }
            Event::UserEvent(UiEvent::Done(result)) => {
                let _ = webview.load_html(&page_result(&result));
            }
            Event::WindowEvent { event: WindowEvent::CloseRequested, .. } => {
                *control_flow = ControlFlow::Exit;
            }
            _ => (),
        }
    });
}
