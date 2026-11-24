import QtQuick
import QtQuick.Layouts
import QtQuick.Controls
import QtQuick.Controls.Material

Item {
    id: dialog
    property alias header: headerLabel.text
    property alias title: titleLabel.text
    property alias buttonText: okButton.text
    property alias buttonEnabled: okButton.enabled
    property alias buttonVisible: okButton.visible
    property Item restoreFocusItem
    default property Item mainItem: null

    signal accepted()
    signal rejected()

    function close() {
        root.closeDialog();
    }

    Keys.onEscapePressed: close()

    Keys.onMenuPressed: {
        if (okButton.enabled)
            okButton.clicked()
    }

    StackView.onDeactivating: {
        restoreFocusItem = Window.window.activeFocusItem;
    }

    StackView.onActivated: {
        if (!restoreFocusItem) {
            let item = mainItem.nextItemInFocusChain();
            if (item)
                item.forceActiveFocus(Qt.TabFocusReason);
        } else {
            restoreFocusItem.forceActiveFocus(Qt.TabFocusReason);
            restoreFocusItem = null;
        }
    }

    onMainItemChanged: {
        if (mainItem) {
            mainItem.parent = contentItem;
            mainItem.anchors.fill = contentItem;
        }
    }

    ToolBar {
        id: toolBar
        anchors {
            top: parent.top
            left: parent.left
            right: parent.right
        }
        height: 88

        RowLayout {
            anchors {
                fill: parent
                leftMargin: 10
                rightMargin: 10
            }

            Button {
                Layout.fillHeight: true
                Layout.preferredWidth: 72
                flat: true
                text: "❮"
                focusPolicy: Qt.NoFocus
                Material.roundedScale: Material.SmallScale
                onClicked: {
                    dialog.rejected();
                    dialog.close();
                }
            }

            Item { Layout.fillWidth: true }

            Button {
                id: okButton
                Layout.fillHeight: true
                flat: true
                padding: 30
                font.pixelSize: 25
                focusPolicy: Qt.NoFocus
                Material.roundedScale: Material.SmallScale
                onClicked: dialog.accepted()
                icon.source: "qrc:/icons/options.svg";
                icon.width: 50
                icon.height: 50
            }
        }

        Column {
            anchors.centerIn: parent
            width: Math.max(0, Math.min(parent.width - 180, 760))
            spacing: 2
            Label {
                id: titleLabel
                width: parent.width
                horizontalAlignment: Qt.AlignHCenter
                font.bold: true
                font.pixelSize: 23
                elide: Text.ElideRight
            }
            Label {
                id: headerLabel
                width: parent.width
                horizontalAlignment: Qt.AlignHCenter
                color: "#a8bdd7"
                font.pixelSize: 12
                elide: Text.ElideRight
                visible: text.length > 0
            }
        }
    }

    Item {
        id: contentItem
        anchors {
            top: toolBar.bottom
            left: parent.left
            right: parent.right
            bottom: parent.bottom
        }
    }
}
