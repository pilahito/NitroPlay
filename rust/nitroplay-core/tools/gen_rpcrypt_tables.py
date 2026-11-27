#!/usr/bin/env python3
"""Extrae las tablas de claves de lib/src/rpcrypt.c a src/rpcrypt_tables.rs.
Uso: python3 tools/gen_rpcrypt_tables.py ../../lib/src/rpcrypt.c > src/rpcrypt_tables.rs"""
import re, sys
src = open(sys.argv[1], encoding='utf-8').read()
out = ["// GENERADO por tools/gen_rpcrypt_tables.py desde lib/src/rpcrypt.c. No editar a mano.",
       "// Licencia de origen: AGPL-3.0-only con excepcion OpenSSL (ver lib/src/rpcrypt.c).", ""]
# funcion actual = ultimo identificador chiaki_rpcrypt_* o bright_ambassador antes de la tabla
func_re = re.compile(r'(?:chiaki_rpcrypt_(\w+)|static ChiakiErrorCode (bright_ambassador))\(')
tab_re = re.compile(r'static const uint8_t (\w+)\[[^\]]*\]\s*=\s*\{([^}]*)\};', re.S)
pos = 0
seen = {}
for m in tab_re.finditer(src):
    name = m.group(1)
    if name.startswith('echo') or name.startswith('hmac') or name.startswith('regist_aes'):
        continue
    funcs = list(func_re.finditer(src, 0, m.start()))
    f = (funcs[-1].group(1) or funcs[-1].group(2)) if funcs else 'global'
    data = [int(x, 16) for x in re.findall(r'0x([0-9a-fA-F]{2})', m.group(2))]
    rname = f"{f}_{name}".upper()
    assert rname not in seen, rname
    seen[rname] = len(data)
    body = ",".join(f"0x{b:02x}" for b in data)
    out.append(f"pub static {rname}: [u8; {len(data)}] = [{body}];")
sys.stderr.write("tablas: " + ", ".join(f"{k}={v}" for k, v in seen.items()) + "\n")
print("\n".join(out))
