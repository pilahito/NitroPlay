//! Bindings crudos a la libreria C de referencia (solo pruebas de equivalencia de `nitroplay-core`).
#![allow(non_camel_case_types, non_snake_case, non_upper_case_globals, dead_code, clippy::all)]
include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::CStr;

    #[test]
    fn la_libreria_arranca_y_describe_errores() {
        unsafe {
            assert_eq!(chiaki_lib_init(), ChiakiErrorCode_CHIAKI_ERR_SUCCESS);
            let s = CStr::from_ptr(chiaki_error_string(ChiakiErrorCode_CHIAKI_ERR_SUCCESS));
            assert!(!s.to_bytes().is_empty());
        }
    }

    #[test]
    fn los_tamanos_de_las_estructuras_existen() {
        assert!(std::mem::size_of::<ChiakiRegistInfo>() > 0);
        assert!(std::mem::size_of::<ChiakiConnectInfo>() > 0);
        assert!(std::mem::size_of::<ChiakiSession>() > 0);
    }
}
