//! El PIN de vinculacion: 8 digitos, de un solo uso. Nunca se guarda.

#[derive(Debug, PartialEq, Eq)]
pub enum PinError {
    Length(usize),
    NotDigits,
}

/// Valida el PIN y lo devuelve como numero (el registro lo usa asi).
pub fn parse_pin(s: &str) -> Result<u32, PinError> {
    let s = s.trim();
    if s.len() != 8 {
        return Err(PinError::Length(s.len()));
    }
    if !s.bytes().all(|b| b.is_ascii_digit()) {
        return Err(PinError::NotDigits);
    }
    s.parse().map_err(|_| PinError::NotDigits)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pines_validos_e_invalidos() {
        assert_eq!(parse_pin("00012345"), Ok(12345));
        assert_eq!(parse_pin(" 12345678 "), Ok(12345678));
        assert_eq!(parse_pin("1234"), Err(PinError::Length(4)));
        assert_eq!(parse_pin("1234567a"), Err(PinError::NotDigits));
        assert_eq!(parse_pin("123456789"), Err(PinError::Length(9)));
    }
}
