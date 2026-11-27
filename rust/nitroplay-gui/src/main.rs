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
    window::{Icon, WindowBuilder},
};
use wry::WebViewBuilder;

const UI: &str = include_str!("ui.html");

enum UiEvent {
    Login,
    Bg(String),
    Consoles(String),
    Code(String),
    Done(Result<String, String>),
    Reg(Result<String, String>),
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

/// Ajustes de calidad: un objeto JSON corto con un conjunto cerrado de caracteres (se inyecta en la pagina).
fn valid_quality(q: &str) -> bool {
    q.starts_with('{')
        && q.ends_with('}')
        && q.len() <= 512
        && q.chars().all(|c| c.is_ascii_alphanumeric() || "{}\":,._ -".contains(c))
}

/// Carpeta de fondos del usuario (se crea si no existe).
fn wallpapers_dir() -> Option<std::path::PathBuf> {
    let base = std::env::var_os("APPDATA")?;
    let dir = std::path::PathBuf::from(base).join("NitroPlay").join("fondos");
    std::fs::create_dir_all(&dir).ok()?;
    Some(dir)
}

/// Escapa un texto para meterlo entre comillas dobles en JavaScript (conserva tildes).
fn js_str(s: &str) -> String {
    s.chars()
        .flat_map(|c| match c {
            '\\' => vec!['\\', '\\'],
            '"' => vec!['\\', '"'],
            '<' => "\\u003c".chars().collect(),
            c if c.is_control() => vec![' '],
            c => vec![c],
        })
        .collect()
}

fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}

/// Mensaje claro para el usuario segun el fallo del registro.
fn regist_error_text(e: &nitroplay_core::regist::RegistError) -> String {
    use nitroplay_core::regist::RegistError as E;
    match e {
        E::BadAccountId => "Primero inicia sesión con PSN (pestaña Cuenta PSN): no hay Account-ID válido.".into(),
        E::BadHost => "La dirección de la consola no es válida.".into(),
        E::Timeout => "La consola no respondió. Comprueba que está encendida, en la misma red y con la pantalla de vincular abierta.".into(),
        E::Http { status, reason } => format!("La consola rechazó el registro (HTTP {status}{}). El PIN suele haber caducado o ser incorrecto: genera uno nuevo.", reason.map(|r| format!(", motivo {r:#x}")).unwrap_or_default()),
        E::InvalidResponse => "La consola respondió algo inesperado.".into(),
        E::Network(m) => format!("Error de red: {}", js_safe(m)),
        E::Failed => "No se pudo preparar la petición de registro.".into(),
        E::Canceled => "Registro cancelado.".into(),
    }
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
    let quality = get_key("quality").filter(|q| valid_quality(q)).unwrap_or_else(|| "{}".into());
    UI.replace("__QUALITY__", &quality)
        .replace("__THEME__", &theme)
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

/// Icono oficial de NitroPlay (gui/nitroplay.ico, 256x256), en RGBA crudo.
fn make_icon() -> Option<Icon> {
    Icon::from_rgba(include_bytes!("icon-256.rgba").to_vec(), 256, 256).ok()
}

// ---- deteccion de consolas en la red (protocolo SRCH de chiaki: UDP a 987 PS4 / 9302 PS5) ----
struct Console {
    name: String,
    ip: String,
    kind: String,
    state: &'static str,
}

fn clean(s: &str) -> String {
    s.chars().filter(|c| c.is_ascii_alphanumeric() || " .-_()".contains(*c)).take(40).collect()
}

fn parse_response(data: &[u8], ip: String) -> Option<Console> {
    let text = String::from_utf8_lossy(data);
    let mut lines = text.lines();
    let first = lines.next()?;
    let mut parts = first.split_whitespace();
    if parts.next()? != "HTTP/1.1" {
        return None;
    }
    let state = match parts.next()? {
        "200" => "Encendida",
        "620" => "En reposo",
        _ => "Estado desconocido",
    };
    let (mut name, mut kind) = (String::new(), String::new());
    for line in lines {
        if let Some((k, v)) = line.split_once(':') {
            match k.trim() {
                "host-name" => name = clean(v.trim()),
                "host-type" => kind = clean(v.trim()),
                _ => {}
            }
        }
    }
    Some(Console { name, ip: clean(&ip), kind, state })
}

/// Broadcast de la subred (asume /24) de la interfaz que usa la ruta a Internet. No envia nada.
fn subnet_broadcast() -> Option<String> {
    let s = std::net::UdpSocket::bind("0.0.0.0:0").ok()?;
    s.connect("8.8.8.8:80").ok()?;
    match s.local_addr().ok()?.ip() {
        std::net::IpAddr::V4(ip) => {
            let o = ip.octets();
            Some(format!("{}.{}.{}.255", o[0], o[1], o[2]))
        }
        _ => None,
    }
}

fn scan_consoles() -> Vec<Console> {
    use std::net::UdpSocket;
    use std::time::{Duration, Instant};
    let socket = (9303u16..=9319)
        .find_map(|p| UdpSocket::bind(("0.0.0.0", p)).ok())
        .or_else(|| UdpSocket::bind(("0.0.0.0", 0)).ok());
    let Some(socket) = socket else { return vec![] };
    let _ = socket.set_broadcast(true);
    let _ = socket.set_read_timeout(Some(Duration::from_millis(250)));
    let deadline = Instant::now() + Duration::from_secs(4);
    let mut last_send: Option<Instant> = None;
    let mut found: Vec<Console> = Vec::new();
    let mut buf = [0u8; 2048];
    while Instant::now() < deadline {
        if last_send.map_or(true, |t| t.elapsed() >= Duration::from_secs(1)) {
            for (port, ver) in [(987u16, "00020020"), (9302u16, "00030010")] {
                let pkt = format!("SRCH * HTTP/1.1\ndevice-discovery-protocol-version:{ver}\n");
                let _ = socket.send_to(pkt.as_bytes(), ("255.255.255.255", port));
                if let Some(b) = subnet_broadcast() {
                    let _ = socket.send_to(pkt.as_bytes(), (b.as_str(), port));
                }
            }
            last_send = Some(Instant::now());
        }
        if let Ok((n, addr)) = socket.recv_from(&mut buf) {
            if let Some(c) = parse_response(&buf[..n], addr.ip().to_string()) {
                if !found.iter().any(|f| f.ip == c.ip) {
                    found.push(c);
                }
            }
        }
    }
    found
}

fn consoles_json(list: &[Console]) -> String {
    let items: Vec<String> = list
        .iter()
        .map(|c| {
            format!(
                "{{\"n\":\"{}\",\"ip\":\"{}\",\"t\":\"{}\",\"s\":\"{}\"}}",
                c.name, c.ip, c.kind, c.state
            )
        })
        .collect();
    format!("[{}]", items.join(","))
}

fn main() {
    let event_loop = EventLoopBuilder::<UiEvent>::with_user_event().build();
    let proxy = event_loop.create_proxy();
    let window = WindowBuilder::new()
        .with_title("NitroPlay")
        .with_maximized(true)
        .with_window_icon(make_icon())
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
            } else if msg == "scan" {
                let p = ipc_proxy.clone();
                std::thread::spawn(move || {
                    let _ = p.send_event(UiEvent::Consoles(consoles_json(&scan_consoles())));
                });
            } else if let Some(arg) = msg.strip_prefix("reg:") {
                // reg:<ip>|<ps5|ps4>|<pin de 8 digitos>
                let parts: Vec<&str> = arg.split('|').collect();
                let p = ipc_proxy.clone();
                let res: Result<(String, bool, u32, String), String> = (|| {
                    if parts.len() != 3 {
                        return Err("Petición de vinculación inválida.".to_string());
                    }
                    let ip: std::net::IpAddr = parts[0].parse().map_err(|_| "La dirección de la consola no es válida.".to_string())?;
                    let pin = nitroplay_core::pin::parse_pin(parts[2]).map_err(|_| "El PIN tiene que ser de 8 dígitos.".to_string())?;
                    let account = get_key("account").filter(|a| a.chars().next() != Some('_')).ok_or("Primero inicia sesión con PSN (pestaña Cuenta PSN).".to_string())?;
                    Ok((ip.to_string(), parts[1] == "ps5", pin, account))
                })();
                match res {
                    Err(e) => {
                        let _ = p.send_event(UiEvent::Reg(Err(e)));
                    }
                    Ok((ip, is_ps5, pin, account)) => {
                        std::thread::spawn(move || {
                            let req = nitroplay_core::regist::RegistRequest { console_ip: &ip, is_ps5, account_id_b64: &account, pin };
                            let out = match nitroplay_core::regist::register(&req) {
                                Ok(c) => {
                                    // Claves de la consola: solo en este equipo, junto al Account-ID.
                                    let v = format!("{}|{}|{}|{}|{}", if is_ps5 { "ps5" } else { "ps4" }, hex(&c.server_mac), hex(&c.rp_regist_key), hex(&c.rp_key), c.rp_key_type);
                                    set_key(&format!("console-{ip}"), &v);
                                    Ok(if c.nickname.is_empty() { ip } else { js_safe(&c.nickname) })
                                }
                                Err(e) => Err(regist_error_text(&e)),
                            };
                            let _ = p.send_event(UiEvent::Reg(out));
                        });
                    }
                }
            } else if msg == "open:fondos" {
                if let Some(dir) = wallpapers_dir() {
                    let _ = std::process::Command::new("explorer").arg(dir).spawn();
                }
            } else if let Some(q) = msg.strip_prefix("quality:") {
                if valid_quality(q) {
                    set_key("quality", q);
                }
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
            Event::UserEvent(UiEvent::Consoles(j)) => {
                let _ = webview.evaluate_script(&format!("showConsoles({j})"));
            }
            Event::UserEvent(UiEvent::Reg(r)) => {
                let (ok, txt) = match r { Ok(n) => (true, format!("Consola vinculada: {n}")), Err(e) => (false, e) };
                let _ = webview.evaluate_script(&format!("regDone({ok},\"{}\")", js_str(&txt)));
            }
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
