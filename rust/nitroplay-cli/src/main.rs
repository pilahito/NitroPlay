//! nitroplay-psn: abre el login de PSN, saca el `code` de la URL final y calcula el Account-ID.
use psn_account::{
    account_id_base64, basic_auth_header, code_from_redirect_url, login_url, parse_access_token,
    parse_user_id, token_body, TOKEN_URL,
};
use std::io::{self, Write};
use std::process::Command;

fn open_browser(url: &str) -> bool {
    Command::new("rundll32")
        .args(["url.dll,FileProtocolHandler", url])
        .spawn()
        .is_ok()
}

fn ask(prompt: &str) -> String {
    print!("{prompt}");
    let _ = io::stdout().flush();
    let mut line = String::new();
    let _ = io::stdin().read_line(&mut line);
    line.trim().to_string()
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
    let url = login_url();
    println!("NitroPlay - login de PSN");
    println!("1) Se abre el navegador. Inicia sesion tu mismo en Sony.");
    println!("2) Al terminar, la pagina final no cargara: copia la URL completa de la barra de direcciones.\n");
    let sin_navegador = std::env::args().any(|a| a == "--sin-navegador");
    if !sin_navegador && !open_browser(&url) {
        println!("No pude abrir el navegador. Abre esta direccion a mano:\n{url}\n");
    }
    let pasted = ask("Pega aqui la URL final (o solo el code): ");
    let code = code_from_redirect_url(&pasted)
        .or_else(|| if !pasted.is_empty() && !pasted.contains(['?', '/', ' ']) { Some(pasted.clone()) } else { None });
    let Some(code) = code else {
        eprintln!("No encontre ningun code en lo que pegaste.");
        std::process::exit(2);
    };
    match fetch_account_id(&code) {
        Ok(id) => println!("\nAccount-ID para registrar la consola: {id}"),
        Err(e) => {
            eprintln!("\nError: {e}");
            std::process::exit(1);
        }
    }
}
