import QtQuick
import QtQuick.Layouts
import QtQuick.Controls
import QtQuick.Controls.Material

import org.nitroplay

Pane {
    id: consolePane
    padding: 0
    property bool showInternetConsoles: false
    readonly property bool hasVisibleConsole: {
        const entries = Chiaki.hosts || []
        for (let i = 0; i < entries.length; ++i) {
            if (entries[i].display && (!entries[i].duid || showInternetConsoles)) return true
        }
        return false
    }
    background: GameBackdrop {
        themeKey: root.currentThemeKey
        themePalette: root.currentTheme
        showLabel: false
    }

    StackView.onActivated: {
        forceActiveFocus(Qt.TabFocusReason)
        if (!Chiaki.autoConnect && !root.initialAsk && !Chiaki.window.directStream) {
            root.initialAsk = true
            if (Chiaki.settings.addSteamShortcutAsk && typeof Chiaki.createSteamShortcut === "function")
                root.showRemindDialog(qsTr("Official Steam artwork + controller layout"), qsTr("Would you like to either create a new non-Steam game for NitroPlay\nor update an existing non-Steam game with the official artwork and controller layout?") + "\n\n" + qsTr("(Note: If you select no now and want to do this later, click the button or press R3 from the main menu.)"), false, () => root.showSteamShortcutDialog(true))
            else if (Chiaki.settings.remotePlayAsk) {
                if (!Chiaki.settings.psnRefreshToken || !Chiaki.settings.psnAuthToken || !Chiaki.settings.psnAuthTokenExpiry || !Chiaki.settings.psnAccountId)
                    root.showRemindDialog(qsTr("Remote Play via PSN"), qsTr("Would you like to connect to PSN?\nThis enables:\n- Automatic registration\n- Playing outside of your home network without port forwarding?") + "\n\n" + qsTr("(Note: If you select no now and want to do this later, go to the Config section of the settings.)"), true, () => root.showPSNTokenDialog(false))
                else
                    Chiaki.settings.remotePlayAsk = false
            }
        }
    }

    function moveSelection(direction) {
        if (!hostsView.count) return
        for (let i = 0; i < hostsView.count; ++i) {
            if (direction > 0) hostsView.incrementCurrentIndex()
            else hostsView.decrementCurrentIndex()
            if (hostsView.currentItem && hostsView.currentItem.visible) break
        }
    }

    Keys.onUpPressed: moveSelection(-1)
    Keys.onDownPressed: moveSelection(1)
    Keys.onMenuPressed: root.showSettingsDialog()
    Keys.onReturnPressed: if (hostsView.currentItem) hostsView.currentItem.connectToHost()
    Keys.onYesPressed: if (hostsView.currentItem) hostsView.currentItem.wakeUpHost()
    Keys.onNoPressed: if (hostsView.currentItem) hostsView.currentItem.deleteHost()
    Keys.onEscapePressed: root.showConfirmDialog(qsTr("Quit"), qsTr("Are you sure you want to quit?"), () => Qt.quit())
    Keys.onPressed: (event) => {
        if (event.modifiers) return
        switch (event.key) {
        case Qt.Key_PageUp:
            if (hostsView.currentItem) hostsView.currentItem.setConsolePin()
            event.accepted = true
            break
        case Qt.Key_PageDown:
            if (Chiaki.settings.psnAuthToken) Chiaki.refreshPsnToken()
            event.accepted = true
            break
        case Qt.Key_F1:
            if (typeof Chiaki.createSteamShortcut === "function") root.showSteamShortcutDialog(false)
            event.accepted = true
            break
        case Qt.Key_F2:
            root.showManualHostDialog()
            event.accepted = true
            break
        }
    }

    Rectangle {
        id: toolbar
        anchors { top: parent.top; left: parent.left; right: parent.right }
        height: 84
        color: "#d90b1424"
        Rectangle { anchors.bottom: parent.bottom; width: parent.width; height: 1; color: "#304465" }

        RowLayout {
            anchors { fill: parent; leftMargin: toolbar.width < 600 ? 12 : 24; rightMargin: toolbar.width < 600 ? 12 : 24 }
            spacing: toolbar.width < 600 ? 6 : 14

            Rectangle {
                color: "#f5f9ff"
                radius: 12
                Layout.preferredWidth: toolbar.width < 600 ? 145 : 206
                Layout.preferredHeight: 55
                Image {
                    anchors { fill: parent; margins: 9 }
                    source: "qrc:/icons/nitroplay-logo.png"
                    sourceSize: Qt.size(200, 60)
                    fillMode: Image.PreserveAspectFit
                }
            }
            Label {
                text: qsTr("TU PS5, EN CUALQUIER PANTALLA")
                color: "#9db2cc"
                font.pixelSize: 12
                font.letterSpacing: 2
                visible: toolbar.width > 1200
            }
            Item { Layout.fillWidth: true }
            Button {
                text: toolbar.width > 720 ? qsTr("Añadir consola") : qsTr("Añadir")
                icon.source: "qrc:/icons/add-24px.svg"
                onClicked: root.showManualHostDialog()
                ToolTip.visible: hovered
                ToolTip.text: qsTr("Añadir consola por IP · F2")
            }
            Button {
                text: qsTr("Actualizar PSN")
                visible: Chiaki.settings.psnAuthToken && toolbar.width > 1080
                onClicked: Chiaki.refreshPsnToken()
            }
            Button {
                icon.source: "qrc:/icons/settings-20px.svg"
                icon.width: 24
                icon.height: 24
                text: toolbar.width > 690 ? qsTr("Ajustes") : ""
                onClicked: root.showSettingsDialog()
                ToolTip.visible: hovered
                ToolTip.text: qsTr("Ajustes")
            }
        }
    }

    ColumnLayout {
        anchors {
            top: toolbar.bottom
            bottom: footer.top
            left: parent.left
            right: parent.right
            topMargin: 28
            leftMargin: Math.max(20, Math.min(72, parent.width * 0.06))
            rightMargin: Math.max(20, Math.min(72, parent.width * 0.06))
        }
        spacing: 16

        ColumnLayout {
            spacing: 4
            Label {
                text: qsTr("Tus consolas")
                font.pixelSize: 30
                font.bold: true
                color: "#f5f9ff"
            }
            Label {
                text: qsTr("Selecciona una consola para empezar a jugar")
                font.pixelSize: 15
                color: "#a1b4ce"
            }
        }

        ListView {
            id: hostsView
            Layout.fillWidth: true
            Layout.fillHeight: true
            clip: true
            spacing: 12
            keyNavigationWraps: true
            model: Chiaki.hosts
            onCountChanged: if (count && currentIndex < 0) currentIndex = 0

            delegate: ItemDelegate {
                id: card
                required property var modelData
                required property int index
                visible: modelData.display && (!modelData.duid || consolePane.showInternetConsoles)
                width: ListView.view ? ListView.view.width : 0
                height: visible ? 148 : 0
                highlighted: ListView.isCurrentItem
                onClicked: connectToHost()

                function connectToHost() {
                    if (modelData.discovered)
                        Chiaki.connectToHost(index, modelData.name)
                    else
                        Chiaki.connectToHost(index)
                }
                function wakeUpHost() {
                    if (!modelData.discovered && !modelData.duid) Chiaki.wakeUpHost(index)
                }
                function deleteHost() {
                    if (modelData.manual)
                        root.showConfirmDialog(qsTr("Delete Console"), qsTr("Are you sure you want to delete this console?"), () => Chiaki.deleteHost(index))
                    else if (modelData.discovered && !modelData.registered)
                        root.showConfirmDialog(qsTr("Hide Console"), qsTr("Are you sure you want to hide this console?") + "\n\n" + qsTr("Note: You can unhide from the Consoles section of the Settings under Hidden Consoles"), () => Chiaki.hideHost(modelData.mac, modelData.name))
                }
                function setConsolePin() { root.showConsolePinDialog(index) }

                background: Rectangle {
                    color: card.highlighted ? "#233c60" : "#e5122033"
                    radius: 18
                    border.color: card.highlighted ? root.currentTheme.accent : "#364f70"
                    border.width: card.highlighted ? 2 : 1
                }

                contentItem: RowLayout {
                    spacing: 18
                    Rectangle {
                        color: "#12233c"
                        radius: 14
                        border.width: 1
                        border.color: "#4776ab"
                        Layout.preferredWidth: card.width < 600 ? 68 : 92
                        Layout.preferredHeight: card.width < 600 ? 68 : 92
                        Label {
                            anchors.centerIn: parent
                            text: card.modelData.ps5 ? "PS5" : "PS4"
                            color: root.currentTheme.accent
                            font.pixelSize: 24
                            font.bold: true
                        }
                    }
                    ColumnLayout {
                        Layout.fillWidth: true
                        spacing: 6
                        Label {
                            text: card.modelData.name || (card.modelData.ps5 ? "PlayStation 5" : "PlayStation 4")
                            color: "#ffffff"
                            font.pixelSize: 23
                            font.bold: true
                            elide: Text.ElideRight
                            Layout.fillWidth: true
                        }
                        Label {
                            text: card.modelData.duid ? qsTr("Conexión por Internet") : card.modelData.discovered ? qsTr("Disponible en la red local") : qsTr("Consola guardada")
                            color: "#b1c3dc"
                            font.pixelSize: 14
                            Layout.fillWidth: true
                            elide: Text.ElideRight
                        }
                        Label {
                            text: card.modelData.address && !Chiaki.settings.streamerMode ? card.modelData.address : (card.modelData.app || "")
                            visible: text.length > 0 && card.width > 540
                            color: "#869db9"
                            font.pixelSize: 12
                            Layout.fillWidth: true
                            elide: Text.ElideRight
                        }
                    }
                    ColumnLayout {
                        spacing: 6
                        Label {
                            Layout.alignment: Qt.AlignRight
                            text: card.modelData.state === "standby" ? qsTr("En reposo") : card.modelData.discovered ? qsTr("En línea") : qsTr("Guardada")
                            color: card.modelData.discovered && card.modelData.state !== "standby" ? "#72e5b7" : "#f7c57d"
                            font.pixelSize: 13
                            font.bold: true
                        }
                        Button {
                            text: card.modelData.state === "standby" ? qsTr("Despertar y jugar") : qsTr("Jugar ahora")
                            highlighted: true
                            onClicked: card.connectToHost()
                        }
                    }
                    Button {
                        text: "⋯"
                        font.pixelSize: 24
                        onClicked: cardActions.open()
                        ToolTip.visible: hovered
                        ToolTip.text: qsTr("Opciones de la consola")
                        Menu {
                            id: cardActions
                            MenuItem { text: qsTr("Despertar"); enabled: !card.modelData.discovered && !card.modelData.duid; onTriggered: card.wakeUpHost() }
                            MenuItem { text: qsTr("Actualizar PIN"); enabled: card.modelData.registered; onTriggered: card.setConsolePin() }
                            MenuItem { text: card.modelData.manual ? qsTr("Eliminar") : qsTr("Ocultar"); enabled: card.modelData.manual || (card.modelData.discovered && !card.modelData.registered); onTriggered: card.deleteHost() }
                        }
                    }
                }
            }
        }
    }

    Column {
        anchors.centerIn: parent
        visible: !consolePane.hasVisibleConsole
        spacing: 16
        Rectangle {
            width: 300
            height: 94
            anchors.horizontalCenter: parent.horizontalCenter
            color: "#f5f9ff"
            radius: 18
            Image {
                anchors { fill: parent; margins: 14 }
                fillMode: Image.PreserveAspectFit
                source: "qrc:/icons/nitroplay-logo.png"
                sourceSize: Qt.size(width, height)
            }
        }
        Label {
            anchors.horizontalCenter: parent.horizontalCenter
            text: consolePane.showInternetConsoles ? qsTr("Buscando tu consola…") : qsTr("No hay consolas locales visibles")
            color: "#f5f9ff"
            font.pixelSize: 23
        }
        Label {
            anchors.horizontalCenter: parent.horizontalCenter
            text: qsTr("Enciende la PS5, comprueba su IP o usa Acceso remoto")
            color: "#a1b4ce"
            font.pixelSize: 15
            wrapMode: Text.WordWrap
            width: Math.min(consolePane.width - 24, 440)
            horizontalAlignment: Text.AlignHCenter
        }
    }

    RowLayout {
        id: footer
        anchors { left: parent.left; right: parent.right; bottom: parent.bottom; leftMargin: 24; rightMargin: 24; bottomMargin: 12 }
        Switch {
            text: consolePane.width < 700 ? qsTr("Buscar PS5") : qsTr("Buscar consolas en la red")
            checked: Chiaki.discoveryEnabled
            onToggled: Chiaki.discoveryEnabled = checked
        }
        Item { Layout.fillWidth: true }
        Button {
            text: consolePane.width < 700 ? qsTr("Internet") : consolePane.showInternetConsoles ? qsTr("Ocultar acceso remoto") : qsTr("Acceso remoto")
            flat: true
            onClicked: consolePane.showInternetConsoles = !consolePane.showInternetConsoles
            ToolTip.visible: hovered
            ToolTip.text: qsTr("Mostrar consolas disponibles por Internet")
        }
        Label { text: "NitroPlay · " + Qt.application.version; visible: consolePane.width >= 700; color: "#8aa0bb"; font.pixelSize: 12 }
    }
}
