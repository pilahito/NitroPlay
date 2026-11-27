//! Lógica pura del Account-ID de PSN (sin red), igual que gui/src/psnaccountid.cpp.
//! La parte HTTP se queda en la app; aquí solo se construye y se transforma.

/// Client ID y secreto del cliente de escritorio (gui/include/psnaccountid.h).
pub const CLIENT_ID: &str = "ba495a24-818c-472b-b12d-ff231c1b5745";
pub const CLIENT_SECRET: &str = "mvaiZkRsAsI1IBkY";
pub const REDIRECT_URI: &str = "https://remoteplay.dl.playstation.net/remoteplay/redirect";
pub const SCOPE: &str = "psn:clientapp referenceDataService:countryConfig.read pushNotification:webSocket.desktop.connect sessionManager:remotePlaySession.system.update";
pub const TOKEN_URL: &str = "https://auth.api.sonyentertainmentnetwork.com/2.0/oauth/token";

/// Cuerpo del POST para cambiar el código por un token (igual que el C++).
pub fn token_body(code: &str) -> String {
    format!(
        "grant_type=authorization_code&code={}&scope={}&redirect_uri={}&",
        code, SCOPE, REDIRECT_URI
    )
}

/// Cabecera Basic de CLIENT_ID:CLIENT_SECRET, ya en base64.
pub fn basic_auth_header() -> String {
    format!("Basic {}", base64_encode(format!("{}:{}", CLIENT_ID, CLIENT_SECRET).as_bytes()))
}

/// Saca el parámetro `code` de la dirección final de la redirección.
/// Solo acepta el valor sin codificar; devuelve None si no hay código.
pub fn code_from_redirect_url(url: &str) -> Option<String> {
    let query = url.trim().split_once('?')?.1;
    let query = query.split('#').next().unwrap_or("");
    query
        .split('&')
        .filter_map(|pair| pair.split_once('='))
        .find(|(k, _)| *k == "code")
        .map(|(_, v)| v.to_string())
        .filter(|v| !v.is_empty())
}

/// user_id (entero de 64 bits) -> 8 bytes little endian -> base64.
/// Es el Account-ID que pide la PS4/PS5 al registrar.
pub fn account_id_base64(user_id: u64) -> String {
    base64_encode(&user_id.to_le_bytes())
}

/// Extrae "user_id" de la respuesta JSON de Sony (`{"user_id":"123..."}`).
pub fn parse_user_id(json: &str) -> Option<u64> {
    let key = "\"user_id\"";
    let rest = &json[json.find(key)? + key.len()..];
    let rest = rest.trim_start().strip_prefix(':')?.trim_start();
    let rest = rest.strip_prefix('"').unwrap_or(rest);
    let digits: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
    digits.parse().ok()
}

/// Direccion de login de Sony (la misma que usa la app de Android y el cliente de escritorio).
pub fn login_url() -> String {
    format!(
        "https://auth.api.sonyentertainmentnetwork.com/2.0/oauth/authorize?service_entity=urn:service-entity:psn&response_type=code&client_id={}&redirect_uri={}&scope={}&request_locale=es_ES&ui=pr&service_logo=ps&layout_type=popup&smcid=remoteplay&prompt=always&PlatformPrivacyWs1=minimal&",
        CLIENT_ID, REDIRECT_URI, SCOPE
    )
}

/// Extrae "access_token" de la respuesta JSON del token (`{"access_token":"abc",...}`).
pub fn parse_access_token(json: &str) -> Option<String> {
    let key = "\"access_token\"";
    let rest = &json[json.find(key)? + key.len()..];
    let rest = rest.trim_start().strip_prefix(':')?.trim_start().strip_prefix('"')?;
    let token: String = rest.chars().take_while(|c| *c != '"').collect();
    if token.is_empty() { None } else { Some(token) }
}
const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

/// Base64 estándar con relleno '=' (sin dependencias).
fn base64_encode(input: &[u8]) -> String {
    let mut out = String::with_capacity((input.len() + 2) / 3 * 4);
    for chunk in input.chunks(3) {
        let b = [chunk[0], *chunk.get(1).unwrap_or(&0), *chunk.get(2).unwrap_or(&0)];
        let n = ((b[0] as u32) << 16) | ((b[1] as u32) << 8) | (b[2] as u32);
        out.push(ALPHABET[(n >> 18) as usize & 63] as char);
        out.push(ALPHABET[(n >> 12) as usize & 63] as char);
        out.push(if chunk.len() > 1 { ALPHABET[(n >> 6) as usize & 63] as char } else { '=' });
        out.push(if chunk.len() > 2 { ALPHABET[n as usize & 63] as char } else { '=' });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn account_id_matches_python_base64() {
        // user_id 1 -> little endian [1,0,0,0,0,0,0,0] -> "AQAAAAAAAAA="
        assert_eq!(account_id_base64(1), "AQAAAAAAAAA=");
        // Comprobado con: python3 -c "import base64,struct;print(base64.b64encode(struct.pack('<Q', 9047483)).decode())"
        assert_eq!(account_id_base64(9047483), "uw2KAAAAAAA=");
    }

    #[test]
    fn base64_known_vectors() {
        assert_eq!(base64_encode(b""), "");
        assert_eq!(base64_encode(b"f"), "Zg==");
        assert_eq!(base64_encode(b"fo"), "Zm8=");
        assert_eq!(base64_encode(b"foo"), "Zm9v");
        assert_eq!(base64_encode(b"foobar"), "Zm9vYmFy");
    }

    #[test]
    fn code_is_read_from_final_url() {
        let url = "https://remoteplay.dl.playstation.net/remoteplay/redirect?code=ABC123&state=x";
        assert_eq!(code_from_redirect_url(url).as_deref(), Some("ABC123"));
        assert_eq!(code_from_redirect_url("https://x.test/?state=1"), None);
        assert_eq!(code_from_redirect_url("https://x.test/?code="), None);
        assert_eq!(code_from_redirect_url("sin url"), None);
    }

    #[test]
    fn user_id_is_parsed_from_json() {
        assert_eq!(parse_user_id(r#"{"user_id":"9047483"}"#), Some(9047483));
        assert_eq!(parse_user_id(r#"{"user_id": 42 , "x":1}"#), Some(42));
        assert_eq!(parse_user_id(r#"{"otra":1}"#), None);
    }

    #[test]
    fn token_body_matches_cpp_layout() {
        let body = token_body("CODIGO");
        assert!(body.starts_with("grant_type=authorization_code&code=CODIGO&scope=psn:clientapp "));
        assert!(body.ends_with("&redirect_uri=https://remoteplay.dl.playstation.net/remoteplay/redirect&"));
    }

    #[test]
    fn basic_header_is_base64_of_id_and_secret() {
        let h = basic_auth_header();
        assert!(h.starts_with("Basic "));
        assert_eq!(h, format!("Basic {}", base64_encode(b"ba495a24-818c-472b-b12d-ff231c1b5745:mvaiZkRsAsI1IBkY")));
    }
}

#[cfg(test)]
mod edge_tests {
    use super::*;

    #[test]
    fn code_from_redirect_url_empty_query() {
        let url = "https://remoteplay.dl.playstation.net/remoteplay/redirect?";
        assert_eq!(code_from_redirect_url(url), None);
    }

    #[test]
    fn code_from_redirect_url_no_code_param() {
        let url = "https://remoteplay.dl.playstation.net/remoteplay/redirect?state=x";
        assert_eq!(code_from_redirect_url(url), None);
    }

    #[test]
    fn parse_user_id_empty_json() {
        assert_eq!(parse_user_id(""), None);
    }
}


#[cfg(test)]
mod login_tests {
    use super::*;

    #[test]
    fn login_url_has_client_and_redirect() {
        let u = login_url();
        assert!(u.starts_with("https://auth.api.sonyentertainmentnetwork.com/2.0/oauth/authorize?"));
        assert!(u.contains(CLIENT_ID) && u.contains(REDIRECT_URI));
    }

    #[test]
    fn access_token_is_parsed() {
        assert_eq!(parse_access_token(r#"{"access_token":"abc123","x":1}"#).as_deref(), Some("abc123"));
        assert_eq!(parse_access_token(r#"{"access_token":""}"#), None);
        assert_eq!(parse_access_token("{}"), None);
    }
}
