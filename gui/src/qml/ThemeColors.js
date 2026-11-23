.pragma library

// Colores de partida del tema Personalizado (los mismos que "PS5 clásico").
var defaults = { accent: "#53b5ff", background: "#0b0d12", primary: "#17243a" }

// Texto de la interfaz: blanco opaco (Material.foreground con Material.Dark, medido en Qt 6.12). Umbrales WCAG: 4.5 texto, 3 elementos de interfaz.
var TEXT_MIN = 4.5
var ACCENT_MIN = 3.0

// Acepta "#rrggbb" o "#aarrggbb" (formato de Qt). Devuelve [r,g,b,a] (0-255, a 0-1) o null.
function parseColor(value) {
    var m = /^#([0-9a-fA-F]{6}|[0-9a-fA-F]{8})$/.exec(String(value))
    if (!m)
        return null
    var hex = m[1]
    var alpha = 1
    if (hex.length === 8) {
        alpha = parseInt(hex.slice(0, 2), 16) / 255
        hex = hex.slice(2)
    }
    return [parseInt(hex.slice(0, 2), 16), parseInt(hex.slice(2, 4), 16), parseInt(hex.slice(4, 6), 16), alpha]
}

// Compone un color con alpha sobre otro opaco [r,g,b].
function over(top, base) {
    var a = top[3]
    return [0, 1, 2].map(function (i) { return top[i] * a + base[i] * (1 - a) })
}

function luminance(rgb) {
    var c = rgb.map(function (v) {
        var s = v / 255
        return s <= 0.03928 ? s / 12.92 : Math.pow((s + 0.055) / 1.055, 2.4)
    })
    return 0.2126 * c[0] + 0.7152 * c[1] + 0.0722 * c[2]
}

function contrastRatio(a, b) {
    var la = luminance(a), lb = luminance(b)
    var hi = Math.max(la, lb), lo = Math.min(la, lb)
    return (hi + 0.05) / (lo + 0.05)
}

// Heurística, no una certificación de accesibilidad: usa el contraste WCAG entre colores
// planos. Un fondo con alpha se compone sobre negro y el acento sobre el fondo ya compuesto.
// "" si no hay problema; "background" si el texto blanco se lee mal sobre el fondo;
// "accent" si el acento apenas se distingue del fondo. Colores no válidos: "".
function warning(background, accent) {
    var bgRaw = parseColor(background)
    if (!bgRaw)
        return ""
    var bg = over(bgRaw, [0, 0, 0])
    if (contrastRatio([255, 255, 255], bg) < TEXT_MIN)
        return "background"
    var acRaw = parseColor(accent)
    if (acRaw && contrastRatio(over(acRaw, bg), bg) < ACCENT_MIN)
        return "accent"
    return ""
}
