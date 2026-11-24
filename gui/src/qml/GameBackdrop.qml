import QtQuick
import QtQuick.Effects

import org.nitroplay

// Original vector motifs inspired by the selected game, drawn only in
// the console browser. No animation or decoration covers the video stream.
Rectangle {
    id: backdrop
    required property string themeKey
    required property var themePalette
    property bool showLabel: true
    // portada nítida del juego abierto a un lado (solo en pantallas grandes)
    property bool showPoster: true
    // miniatura (vista previa en Ajustes): la foto del juego nítida, sin difuminar
    property bool compact: false
    // movimiento suave de la foto (no en miniaturas)
    property bool animated: !compact
    clip: true
    // con un juego abierto en la PS5, el fondo toma su portada y su color
    readonly property bool gameArt: Chiaki.gameArt.active
    readonly property color accentColor: gameArt ? Chiaki.gameArt.accent : themePalette.accent
    // imagen propia que el usuario eligió para este tema
    readonly property string userImage: Chiaki.settings.themeImagesRevision >= 0 ? Chiaki.settings.themeImage(themeKey) : ""
    // los temas de juegos traen la portada de su juego (se descarga la primera vez)
    readonly property bool isGameTheme: ["ps5", "ps4", "ps3", "ps2", "ps1", "custom", "chiaki"].indexOf(themeKey) < 0
    readonly property string themeCover: isGameTheme && userImage === "" && Chiaki.gameArt.coversRevision >= 0
                                         ? Chiaki.gameArt.coverFor(themePalette.name) : ""
    readonly property string coverSource: gameArt ? Chiaki.gameArt.coverUrl : themeCover
    readonly property bool hasCover: coverSource !== ""
    // fondo oficial de playstation.com: grande, se ve nítido y sin póster
    readonly property bool officialArt: coverSource.indexOf("-oficial.") >= 0
    color: themePalette.background

    Rectangle {
        anchors.fill: parent
        visible: backdrop.themeKey !== "chiaki"
        gradient: Gradient {
            GradientStop { position: 0; color: backdrop.themePalette.primary }
            GradientStop { position: 1; color: backdrop.themePalette.background }
        }
        opacity: 0.4
    }

    // la foto del tema (la del usuario o la del juego), con un movimiento lento
    // de acercamiento y desplazamiento
    Item {
        id: artLayer
        anchors.fill: parent
        property real drift: 0
        transform: [
            Scale {
                origin.x: artLayer.width / 2
                origin.y: artLayer.height / 2
                xScale: backdrop.animated ? 1.06 + 0.05 * artLayer.drift : 1
                yScale: xScale
            },
            Translate { x: backdrop.animated ? (artLayer.drift - 0.5) * artLayer.width * 0.03 : 0 }
        ]
        SequentialAnimation on drift {
            running: backdrop.animated && (themeImage.visible || backdrop.hasCover)
            loops: Animation.Infinite
            NumberAnimation { from: 0; to: 1; duration: 28000; easing.type: Easing.InOutSine }
            NumberAnimation { from: 1; to: 0; duration: 28000; easing.type: Easing.InOutSine }
        }

        Image {
            id: themeImage
            anchors.fill: parent
            readonly property string url: backdrop.userImage
            source: url ? url + "?r=" + Chiaki.settings.themeImagesRevision : ""
            visible: url !== "" && status === Image.Ready && !backdrop.gameArt
            fillMode: Image.PreserveAspectCrop
            asynchronous: true
            cache: false
            sourceSize.width: 2560
            opacity: 0.75
        }

        // portada del juego, ampliada y difuminada como fondo
        Image {
            id: gameCover
            anchors.fill: parent
            source: backdrop.coverSource
            fillMode: Image.PreserveAspectCrop
            asynchronous: true
            visible: false
        }
        MultiEffect {
            anchors.fill: parent
            source: gameCover
            visible: backdrop.hasCover && gameCover.status === Image.Ready
            blurEnabled: !backdrop.officialArt && !backdrop.compact
            blur: 1.0
            blurMax: 48
            saturation: backdrop.officialArt || backdrop.compact ? 0 : 0.15
            opacity: backdrop.officialArt || backdrop.compact ? 1 : 0.85
        }
    }

    // oscurece la imagen para que los textos y botones se sigan leyendo
    Rectangle {
        anchors.fill: parent
        visible: (themeImage.visible || backdrop.hasCover) && !backdrop.compact
        gradient: Gradient {
            GradientStop { position: 0; color: Qt.alpha(backdrop.themePalette.background, 0.35) }
            GradientStop { position: 1; color: Qt.alpha(backdrop.themePalette.background, 0.75) }
        }
    }

    Canvas {
        id: art
        // con foto del juego no hace falta el dibujo de líneas
        anchors.fill: parent
        opacity: 0.22
        visible: backdrop.themeKey !== "chiaki" && !themeImage.visible && !backdrop.hasCover
        onWidthChanged: requestPaint()
        onHeightChanged: requestPaint()
        Connections {
            target: backdrop
            function onThemeKeyChanged() { art.requestPaint(); }
            function onThemePaletteChanged() { art.requestPaint(); }
            function onAccentColorChanged() { art.requestPaint(); }
        }
        onPaint: {
            const ctx = getContext("2d");
            ctx.reset();
            ctx.scale(width / 1000, height / 700);
            ctx.strokeStyle = backdrop.accentColor;
            ctx.fillStyle = backdrop.accentColor;
            ctx.lineWidth = 2;
            function line(points) {
                ctx.beginPath(); ctx.moveTo(points[0][0], points[0][1]);
                for (let i = 1; i < points.length; ++i)
                    ctx.lineTo(points[i][0], points[i][1]);
                ctx.stroke();
            }
            function circle(x, y, radius) {
                ctx.beginPath(); ctx.arc(x, y, radius, 0, Math.PI * 2); ctx.stroke();
            }
            switch (backdrop.themeKey) {
            case "spiderman":
                for (let i = 0; i < 16; ++i) {
                    const a = i * Math.PI / 8;
                    line([[840, 200], [840 + Math.cos(a) * 900, 200 + Math.sin(a) * 900]]);
                }
                for (let r = 90; r < 850; r += 85) circle(840, 200, r);
                break;
            case "gow":
                circle(770, 350, 195); circle(770, 350, 210);
                ctx.lineWidth = 8;
                line([[700, 570], [805, 180]]);
                line([[790, 220], [690, 185], [665, 300], [770, 285]]);
                for (let i = 0; i < 12; ++i) {
                    const a = i * Math.PI / 6;
                    line([[770 + 180 * Math.cos(a), 350 + 180 * Math.sin(a)],
                          [770 + 192 * Math.cos(a), 350 + 192 * Math.sin(a)]]);
                }
                break;
            case "horizon":
                circle(780, 220, 90);
                line([[0, 620], [160, 400], [270, 510], [490, 280], [660, 490], [820, 360], [1000, 560]]);
                line([[0, 690], [220, 540], [420, 610], [640, 420], [1000, 650]]);
                for (let y = 540; y < 700; y += 28) line([[0, y], [1000, y]]);
                break;
            case "tlou":
                for (let stem = 0; stem < 5; ++stem) {
                    const x = 650 + stem * 75;
                    line([[x, 700], [x - 40, 110 + stem * 30]]);
                    for (let y = 220; y < 650; y += 70) {
                        line([[x - 30, y + 25], [x - 85, y - 10], [x - 60, y + 35], [x - 30, y + 25]]);
                        line([[x - 25, y + 50], [x + 30, y + 10], [x + 5, y + 60], [x - 25, y + 50]]);
                    }
                }
                break;
            case "ghost":
                circle(805, 235, 105);
                for (let i = 0; i < 8; ++i) {
                    ctx.beginPath(); ctx.moveTo(100, 390 + i * 35);
                    ctx.bezierCurveTo(350, 220 + i * 40, 600, 620, 1050, 350 + i * 35); ctx.stroke();
                }
                break;
            case "astro":
                circle(790, 350, 150); circle(790, 350, 225);
                for (let i = 0; i < 35; ++i) {
                    const x = (i * 163 + 53) % 1000, y = (i * 97 + 81) % 700;
                    line([[x - 5, y], [x + 5, y]]); line([[x, y - 5], [x, y + 5]]);
                }
                ctx.lineWidth = 9;
                line([[750, 335], [750, 365]]); line([[830, 335], [830, 365]]);
                break;
            case "gt7":
                ctx.lineWidth = 10;
                line([[600, 520], [530, 300], [650, 200], [880, 240], [930, 430], [790, 560], [600, 520]]);
                ctx.lineWidth = 2;
                line([[625, 480], [580, 315], [665, 250], [840, 280], [875, 410], [775, 510], [625, 480]]);
                for (let i = 0; i < 7; ++i) line([[560, 500 + i * 10], [610, 485 + i * 10]]);
                break;
            case "ratchet":
                for (let g = 0; g < 3; ++g) {
                    const x = 650 + g * 100, y = 260 + (g % 2) * 170;
                    circle(x, y, 85); circle(x, y, 30);
                    for (let i = 0; i < 12; ++i) {
                        const a = i * Math.PI / 6;
                        line([[x + 85 * Math.cos(a), y + 85 * Math.sin(a)], [x + 105 * Math.cos(a), y + 105 * Math.sin(a)]]);
                    }
                }
                break;
            case "returnal":
                for (let r = 70; r <= 250; r += 45) circle(780, 350, r);
                for (let i = 0; i < 35; ++i) {
                    const x = (i * 139) % 1000, y = (i * 71) % 700;
                    line([[x, y], [x - 12, y + 75]]);
                }
                break;
            case "uncharted":
                circle(790, 350, 160); circle(790, 350, 180);
                line([[790, 120], [835, 350], [790, 580], [745, 350], [790, 120]]);
                line([[560, 350], [790, 305], [1020, 350], [790, 395], [560, 350]]);
                break;
            case "bloodborne":
                for (let i = 0; i < 6; ++i) {
                    const x = 520 + i * 85, y = 270 + (i % 3) * 60;
                    line([[x - 25, 700], [x - 25, y + 70], [x, y], [x + 25, y + 70], [x + 25, 700]]);
                    line([[x, y], [x, y - 55]]); line([[x - 15, y - 35], [x + 15, y - 35]]);
                }
                circle(800, 170, 80);
                break;
            case "demons":
                circle(790, 350, 200);
                line([[790, 130], [815, 210], [800, 420], [780, 420], [765, 210], [790, 130]]);
                ctx.lineWidth = 7;
                line([[710, 420], [870, 420]]); line([[790, 420], [790, 560]]);
                break;
            case "stranding":
                for (let i = 0; i < 14; ++i) {
                    ctx.beginPath(); ctx.moveTo(420, 90 + i * 35);
                    ctx.bezierCurveTo(690, 120, 580, 620, 1040, 200 + i * 30); ctx.stroke();
                }
                for (let i = 0; i < 9; ++i) line([[600 + i * 42, 0], [600 + i * 42, 140 + (i % 3) * 100]]);
                break;
            case "ps1":
                ctx.lineWidth = 3;
                for (let i = 0; i < 8; ++i) {
                    line([[560 + i * 48, 620], [780, 130 + i * 28], [960, 620]]);
                    line([[560, 620 - i * 32], [960, 620 - i * 32]]);
                }
                circle(780, 330, 82);
                break;
            case "ps2":
                for (let r = 65; r <= 245; r += 45) circle(790, 350, r);
                for (let i = 0; i < 12; ++i) {
                    const a = i * Math.PI / 6;
                    line([[790 + 60 * Math.cos(a), 350 + 60 * Math.sin(a)],
                          [790 + 245 * Math.cos(a), 350 + 245 * Math.sin(a)]]);
                }
                break;
            case "ps3":
                for (let i = 0; i < 11; ++i) {
                    ctx.beginPath(); ctx.moveTo(470, 160 + i * 35);
                    ctx.bezierCurveTo(650, 50 + i * 22, 700, 640 - i * 20, 1020, 320 + i * 18); ctx.stroke();
                }
                circle(790, 350, 125);
                break;
            case "crash":
                for (let i = 0; i < 9; ++i) {
                    const r = 55 + i * 25;
                    line([[770 + r * Math.cos(i * 0.6), 350 + r * Math.sin(i * 0.6)],
                          [770 + (r + 95) * Math.cos(i * 0.6 + 0.8), 350 + (r + 95) * Math.sin(i * 0.6 + 0.8)]]);
                }
                circle(780, 350, 45);
                line([[600, 650], [690, 470], [790, 560], [880, 420], [1000, 640]]);
                break;
            case "spyro":
                line([[790, 350], [650, 230], [550, 170], [610, 300], [720, 390]]);
                line([[790, 350], [930, 230], [1030, 170], [970, 300], [860, 390]]);
                circle(790, 350, 58);
                for (let i = 0; i < 16; ++i) {
                    const x = 570 + ((i * 137) % 420), y = 130 + ((i * 83) % 470);
                    line([[x - 6, y], [x + 6, y]]); line([[x, y - 6], [x, y + 6]]);
                }
                break;
            case "mgs":
                for (let r = 50; r <= 230; r += 45) circle(790, 350, r);
                line([[560, 350], [1020, 350]]); line([[790, 120], [790, 580]]);
                line([[790, 350], [950, 210]]);
                break;
            case "ff7":
                for (let i = 0; i < 12; ++i) {
                    const x = 650 + i * 28;
                    line([[x, 130], [x - 170, 390], [x - 220, 500]]);
                }
                circle(790, 350, 54); circle(790, 350, 82);
                for (let i = 0; i < 18; ++i) {
                    const x = (i * 151 + 31) % 1000, y = (i * 97 + 47) % 700;
                    line([[x - 5, y], [x + 5, y]]);
                }
                break;
            case "tekken3":
                ctx.lineWidth = 6;
                line([[790, 120], [970, 350], [790, 580], [610, 350], [790, 120]]);
                ctx.lineWidth = 2;
                line([[790, 190], [910, 350], [790, 510], [670, 350], [790, 190]]);
                line([[610, 350], [970, 350]]); line([[790, 120], [790, 580]]);
                break;
            case "shadow":
                line([[500, 610], [610, 390], [660, 430], [725, 235], [770, 390], [825, 285], [900, 470], [960, 410], [1040, 620]]);
                for (let i = 0; i < 8; ++i) line([[540 + i * 55, 650], [590 + i * 45, 600]]);
                circle(820, 180, 62);
                break;
            case "ico":
                for (let i = 0; i < 5; ++i) {
                    const x = 610 + i * 88;
                    ctx.beginPath(); ctx.moveTo(x, 590);
                    ctx.lineTo(x, 310); ctx.bezierCurveTo(x, 170, x + 88, 170, x + 88, 310);
                    ctx.lineTo(x + 88, 590); ctx.stroke();
                }
                line([[570, 590], [1040, 590]]);
                break;
            case "lbp":
                circle(790, 350, 190); circle(790, 350, 150);
                for (let i = 0; i < 20; ++i) {
                    const a = i * Math.PI / 10;
                    line([[790 + 154 * Math.cos(a), 350 + 154 * Math.sin(a)],
                          [790 + 186 * Math.cos(a), 350 + 186 * Math.sin(a)]]);
                }
                circle(790, 350, 28);
                break;
            case "infamous":
                line([[650, 120], [735, 290], [690, 330], [815, 340], [740, 580]]);
                line([[900, 120], [820, 270], [865, 315], [745, 360], [865, 580]]);
                for (let i = 0; i < 6; ++i) circle(780, 350, 90 + i * 26);
                break;
            case "killzone":
                ctx.beginPath(); ctx.moveTo(550, 350);
                ctx.bezierCurveTo(650, 210, 900, 210, 1030, 350);
                ctx.bezierCurveTo(900, 490, 650, 490, 550, 350); ctx.stroke();
                circle(790, 350, 76); circle(790, 350, 24);
                line([[560, 350], [1020, 350]]);
                break;
            case "sackboy":
                circle(790, 350, 190);
                for (let i = 0; i < 11; ++i) {
                    const y = 210 + i * 28;
                    line([[620, y], [960, y]]);
                    line([[620 + (i % 2) * 14, y - 14], [620 + (i % 2) * 14, y + 14]]);
                }
                circle(790, 350, 35);
                break;
            case "jak": // orbes y espiral de los Precursores
                for (let i = 0; i < 7; ++i) circle(790, 350, 40 + i * 32);
                for (let i = 0; i < 12; ++i) {
                    const a = i * Math.PI / 6;
                    circle(790 + Math.cos(a) * 270, 350 + Math.sin(a) * 270, 14);
                }
                break;
            case "sly": // luna y tejados de la ciudad de noche
                circle(820, 180, 95);
                line([[480, 700], [480, 470], [560, 420], [640, 470], [640, 380], [720, 330], [800, 380],
                      [800, 450], [880, 410], [960, 450], [960, 360], [1040, 320], [1040, 700]]);
                for (let i = 0; i < 5; ++i) line([[520 + i * 110, 520], [560 + i * 110, 520]]);
                break;
            case "daysgone": // carretera y pinos
                line([[700, 700], [880, 260]]); line([[1060, 700], [920, 260]]);
                for (let i = 0; i < 6; ++i) line([[890 - i * 6, 640 - i * 70], [893 - i * 6, 600 - i * 70]]);
                for (let i = 0; i < 7; ++i) {
                    const x = 470 + i * 60;
                    line([[x, 520], [x + 28, 380], [x + 56, 520]]); line([[x + 28, 520], [x + 28, 570]]);
                }
                break;
            case "detroit": // anillo LED y triángulo
                for (let i = 0; i < 3; ++i) circle(790, 350, 150 + i * 22);
                line([[790, 230], [890, 410], [690, 410], [790, 230]]);
                break;
            case "persona5": // cortes en zigzag y estrella
                ctx.lineWidth = 6;
                for (let i = 0; i < 6; ++i)
                    line([[480 + i * 40, 700], [620 + i * 60, 420], [560 + i * 60, 380], [760 + i * 50, 0]]);
                ctx.lineWidth = 3;
                line([[830, 180], [855, 250], [930, 250], [870, 295], [893, 365], [830, 322],
                      [767, 365], [790, 295], [730, 250], [805, 250], [830, 180]]);
                break;
            case "re": // paraguas de 8 gajos
                circle(790, 350, 210);
                for (let i = 0; i < 8; ++i) {
                    const a = i * Math.PI / 4;
                    line([[790, 350], [790 + Math.cos(a) * 210, 350 + Math.sin(a) * 210]]);
                }
                break;
            case "silenthill": // niebla ondulada
                for (let i = 0; i < 16; ++i) {
                    ctx.beginPath(); ctx.moveTo(380, 120 + i * 36);
                    ctx.bezierCurveTo(560, 90 + i * 36, 760, 160 + i * 36, 1040, 110 + i * 36); ctx.stroke();
                }
                break;
            case "kh": // corona y corazón
                line([[690, 330], [710, 200], [750, 280], [790, 170], [830, 280], [870, 200], [890, 330], [690, 330]]);
                ctx.beginPath(); ctx.moveTo(790, 560);
                ctx.bezierCurveTo(640, 460, 690, 360, 790, 430);
                ctx.bezierCurveTo(890, 360, 940, 460, 790, 560); ctx.stroke();
                break;
            case "stellar": // hoja de espada y estrellas
                ctx.lineWidth = 5;
                line([[600, 620], [960, 120]]); line([[630, 560], [690, 600]]);
                ctx.lineWidth = 2;
                for (let i = 0; i < 24; ++i) circle((i * 167) % 520 + 480, (i * 97) % 640 + 30, 3);
                break;
            case "helldivers": // cápsulas de descenso
                ctx.lineWidth = 4;
                for (let i = 0; i < 7; ++i) {
                    const x = 520 + i * 75, y = 80 + (i % 3) * 120;
                    line([[x, y], [x + 40, y + 180]]);
                    line([[x + 30, y + 190], [x + 50, y + 190], [x + 40, y + 225], [x + 30, y + 190]]);
                }
                break;
            case "wipeout": // pista de carreras y flechas
                for (let i = 0; i < 6; ++i) {
                    ctx.beginPath(); ctx.moveTo(420, 650 - i * 20);
                    ctx.bezierCurveTo(640, 600 - i * 20, 760, 160 + i * 20, 1040, 120 + i * 20); ctx.stroke();
                }
                for (let i = 0; i < 4; ++i) line([[800 + i * 45, 300], [830 + i * 45, 340], [800 + i * 45, 380]]);
                break;
            case "parappa": // notas musicales
                for (let i = 0; i < 7; ++i) {
                    const x = 560 + i * 70, y = 200 + (i % 3) * 110;
                    circle(x, y + 120, 18); line([[x + 18, y + 120], [x + 18, y]]); line([[x + 18, y], [x + 50, y + 25]]);
                }
                break;
            default:
                for (let i = 0; i < 9; ++i) {
                    ctx.beginPath(); ctx.moveTo(-100, 530 + i * 25);
                    ctx.bezierCurveTo(250, 100, 650, 900, 1100, 240 + i * 25); ctx.stroke();
                }
            }
        }
    }
    // la portada nítida del juego, con su nombre
    Column {
        anchors {
            right: parent.right
            verticalCenter: parent.verticalCenter
            rightMargin: Math.round(parent.width * 0.05)
        }
        spacing: 12
        visible: backdrop.hasCover && !backdrop.officialArt && backdrop.showPoster && backdrop.width > 1000 && poster.status === Image.Ready
        Image {
            id: poster
            source: backdrop.coverSource
            height: Math.round(backdrop.height * 0.55)
            fillMode: Image.PreserveAspectFit
            asynchronous: true
            smooth: true
            layer.enabled: true
            layer.effect: MultiEffect {
                shadowEnabled: true
                shadowBlur: 0.8
                shadowOpacity: 0.7
            }
        }
        Text {
            width: poster.paintedWidth
            horizontalAlignment: Text.AlignHCenter
            wrapMode: Text.WordWrap
            text: backdrop.gameArt ? Chiaki.gameArt.gameName : backdrop.themePalette.name
            color: backdrop.accentColor
            font.pixelSize: 18
            font.bold: true
        }
    }

    Text {
        anchors.right: parent.right
        anchors.bottom: parent.bottom
        anchors.margins: 28
        text: backdrop.themePalette.name
        font.pixelSize: 18
        font.letterSpacing: 2
        color: backdrop.accentColor
        opacity: 0.6
        visible: backdrop.showLabel && backdrop.themeKey !== "chiaki"
    }
}
