//! NitroPlay (Windows): interfaz con temas, cuenta PSN integrada y guia de HDR/calidad.
//! El login de Sony va dentro de la ventana: se corta la redireccion, se saca el `code`
//! y se calcula el Account-ID. Todo en Rust (wry + tao).
#![windows_subsystem = "windows"]

use psn_account::{
    account_id_base64, basic_auth_header, code_from_redirect_url, login_url, parse_access_token,
    parse_user_id, token_body, REDIRECT_URI, TOKEN_URL,
};
use std::path::PathBuf;
use tao::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoopBuilder},
    window::WindowBuilder,
};
use wry::WebViewBuilder;

const UI: &str = include_str!("ui.html");

enum UiEvent {
    Login,
    Bg(String),
    Code(String),
    Done(Result<String, String>),
}

// ---- configuracion en %APPDATA%\NitroPlay\config.txt (lineas clave=valor) ----
fn config_path() -> Option<PathBuf> {
    std::env::var_os("APPDATA").map(|a| PathBuf::from(a).join("NitroPlay").join("config.txt"))
}

fn get_key(key: &str) -> Option<String> {
    let text = std::fs::read_to_string(config_path()?).ok()?;
    text.lines()
        .filter_map(|l| l.split_once('='))
        .find(|(k, _)| *k == key)
        .map(|(_, v)| v.trim().to_string())
        .filter(|v| !v.is_empty())
}

fn set_key(key: &str, value: &str) {
    let Some(path) = config_path() else { return };
    let mut lines: Vec<String> = std::fs::read_to_string(&path)
        .unwrap_or_default()
        .lines()
        .filter(|l| l.split_once('=').map(|(k, _)| k != key).unwrap_or(false))
        .map(String::from)
        .collect();
    lines.push(format!("{key}={value}"));
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    let _ = std::fs::write(path, lines.join("\n"));
}

/// Los ids de tema solo llevan letras, numeros y guiones (se validan antes de guardar).
fn valid_theme(id: &str) -> bool {
    !id.is_empty() && id.len() <= 20 && id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
}

/// Fondo propio: solo una imagen en data URL (data:image/...;base64,...) y de tamano razonable.
fn valid_bg(d: &str) -> bool {
    d.starts_with("data:image/")
        && d.len() <= 8_000_000
        && d.chars().all(|c| c.is_ascii_alphanumeric() || "+/=:;,-".contains(c))
}

fn js_safe(s: &str) -> String {
    s.chars()
        .filter(|c| c.is_ascii_alphanumeric() || " .,:;_-+/=()".contains(*c))
        .collect()
}

fn render(tab: &str, error: Option<&str>) -> String {
    let theme = get_key("theme").filter(|t| valid_theme(t)).unwrap_or_else(|| "nitroplay".into());
    let account = get_key("account").map(|a| js_safe(&a)).unwrap_or_else(|| "_".into());
    let error = error.map(js_safe).unwrap_or_else(|| "_".into());
    let bg = get_key(&format!("bg-{theme}")).filter(|d| valid_bg(d)).unwrap_or_else(|| "_".into());
    UI.replace("__THEME__", &theme)
        .replace("__ACCOUNT__", &account)
        .replace("__TAB__", tab)
        .replace("__ERROR__", &error)
        .replace("__BG__", &bg)
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

fn main() {
    let event_loop = EventLoopBuilder::<UiEvent>::with_user_event().build();
    let proxy = event_loop.create_proxy();
    let window = WindowBuilder::new()
        .with_title("NitroPlay")
        .with_maximized(true)
        .build(&event_loop)
        .unwrap();

    let nav_proxy = proxy.clone();
    let ipc_proxy = proxy.clone();
    let webview = WebViewBuilder::new()
        .with_html(render("inicio", None))
        .with_navigation_handler(move |url: String| {
            if url.starts_with(REDIRECT_URI) {
                if let Some(code) = code_from_redirect_url(&url) {
                    let _ = nav_proxy.send_event(UiEvent::Code(code));
                }
                return false; // la pagina final de Sony no hace falta cargarla
            }
            true
        })
        .with_ipc_handler(move |req| {
            let msg: &str = req.body();
            if msg == "login" {
                let _ = ipc_proxy.send_event(UiEvent::Login);
            } else if msg == "open:wallpapers" {
                let _ = std::process::Command::new("rundll32")
                    .args(["url.dll,FileProtocolHandler", "https://www.playstation.com/es-es/wallpapers/"])
                    .spawn();
            } else if msg == "bg-clear" {
                let t = get_key("theme").unwrap_or_else(|| "nitroplay".into());
                set_key(&format!("bg-{t}"), "");
            } else if let Some(d) = msg.strip_prefix("bg:") {
                if valid_bg(d) {
                    let t = get_key("theme").unwrap_or_else(|| "nitroplay".into());
                    set_key(&format!("bg-{t}"), d);
                }
            } else if let Some(id) = msg.strip_prefix("theme:") {
                if valid_theme(id) {
                    set_key("theme", id);
                    let d = get_key(&format!("bg-{id}")).filter(|d| valid_bg(d)).unwrap_or_default();
                    let _ = ipc_proxy.send_event(UiEvent::Bg(d));
                }
            }
        })
        .build(&window)
        .unwrap();

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;
        match event {
            Event::UserEvent(UiEvent::Bg(d)) => {
                let _ = webview.evaluate_script(&format!("applyBg(\"{d}\")"));
            }
            Event::UserEvent(UiEvent::Login) => {
                let _ = webview.load_url(&login_url());
            }
            Event::UserEvent(UiEvent::Code(code)) => {
                let _ = webview.load_html(&render("cuenta", Some("Obteniendo tu Account-ID...")));
                let p = proxy.clone();
                std::thread::spawn(move || {
                    let _ = p.send_event(UiEvent::Done(fetch_account_id(&code)));
                });
            }
            Event::UserEvent(UiEvent::Done(result)) => {
                let html = match result {
                    Ok(id) => {
                        set_key("account", &id);
                        render("cuenta", None)
                    }
                    Err(e) => render("cuenta", Some(&format!("No se pudo obtener el ID: {e}"))),
                };
                let _ = webview.load_html(&html);
            }
            Event::WindowEvent { event: WindowEvent::CloseRequested, .. } => {
                *control_flow = ControlFlow::Exit;
            }
            _ => (),
        }
    });
}
