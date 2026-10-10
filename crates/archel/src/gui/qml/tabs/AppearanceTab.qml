// Apparence tab: blur switch, opacity sliders, wallpaper picker and
// font fields (writable, plan 06-08).
//
// Every write confirms first in the global ConfirmModal (D-12): the
// model workers open the core-verbatim question on the shared session
// and Main.qml renders it. While a write runs only this tab locks
// (D-15: operationBusy disables the controls + BusyIndicator);
// failures surface in the global ErrorModal (D-17).
import QtQuick
import QtQuick.Controls
import QtQuick.Dialogs
import QtQuick.Layouts
import org.archel.settings

Item {
    AppearanceModel {
        id: appearance
    }

    Timer {
        interval: 1000
        running: true
        repeat: true
        onTriggered: appearance.tick()
    }
    Component.onCompleted: appearance.tick()

    FileDialog {
        id: wallpaperPicker
        title: "Choisir un fond d'écran"
        nameFilters: ["Images (*.png *.jpg *.jpeg *.webp *.bmp *.gif)"]
        onAccepted: wallpaperField.text = selectedFile.toString().replace("file://", "")
    }

    ScrollView {
        id: view
        anchors.fill: parent
        anchors.margins: 16
        clip: true
        contentWidth: availableWidth

        ColumnLayout {
            width: view.availableWidth
            spacing: 24

            Label {
                text: "Apparence"
                font.bold: true
                font.pixelSize: Math.round(Qt.font({}).pixelSize * 1.25)
            }

            PollBanner {
                bannerVisible: appearance.bannerVisible
                onDismissRequested: appearance.dismissBanner()
            }

            BusyIndicator {
                running: appearance.operationBusy
                visible: appearance.operationBusy
                Layout.alignment: Qt.AlignHCenter
            }

            GroupBox {
                title: "Effets"
                Layout.fillWidth: true

                ColumnLayout {
                    anchors.fill: parent
                    spacing: 8

                    RowLayout {
                        Layout.fillWidth: true
                        spacing: 8

                        Label {
                            text: "Flou"
                            Layout.fillWidth: true
                        }
                        Switch {
                            id: blurSwitch
                            checked: appearance.blurText === "on"
                            enabled: !appearance.operationBusy
                            onClicked: appearance.setBlur(checked)
                        }
                    }
                    StatField {
                        Layout.fillWidth: true
                        fieldLabel: "État"
                        fieldValue: appearance.blurText
                    }

                    Label {
                        text: "Opacité active : " + Math.round(activeSlider.value * 100) + " %"
                        Layout.fillWidth: true
                    }
                    Slider {
                        id: activeSlider
                        Layout.fillWidth: true
                        from: 0
                        to: 1
                        stepSize: 0.01
                        value: {
                            if (appearance.opacityText === "")
                                return 1;
                            var active = parseFloat(appearance.opacityText.split("/")[0]);
                            return isNaN(active) ? 1 : active;
                        }
                        enabled: !appearance.operationBusy
                    }
                    Label {
                        text: "Opacité inactive : " + Math.round(inactiveSlider.value * 100) + " %"
                        Layout.fillWidth: true
                    }
                    Slider {
                        id: inactiveSlider
                        Layout.fillWidth: true
                        from: 0
                        to: 1
                        stepSize: 0.01
                        value: {
                            if (appearance.opacityText === "")
                                return 1;
                            var parts = appearance.opacityText.split("/");
                            if (parts.length < 2)
                                return 1;
                            var inactive = parseFloat(parts[1]);
                            return isNaN(inactive) ? 1 : inactive;
                        }
                        enabled: !appearance.operationBusy
                    }
                    StatField {
                        Layout.fillWidth: true
                        fieldLabel: "Active / inactive"
                        fieldValue: appearance.opacityText
                    }
                    Button {
                        text: "Appliquer"
                        enabled: !appearance.operationBusy
                        onClicked: appearance.setOpacity(activeSlider.value.toFixed(2), inactiveSlider.value.toFixed(2))
                    }
                }
            }

            GroupBox {
                title: "Fond d'écran"
                Layout.fillWidth: true

                ColumnLayout {
                    anchors.fill: parent
                    spacing: 8

                    StatField {
                        Layout.fillWidth: true
                        fieldLabel: "Chemin"
                        fieldValue: appearance.wallpaperText
                    }
                    RowLayout {
                        Layout.fillWidth: true
                        spacing: 8

                        TextField {
                            id: wallpaperField
                            Layout.fillWidth: true
                            placeholderText: "/chemin/vers/image.png"
                            text: appearance.wallpaperText === "N/A" ? "" : appearance.wallpaperText
                            enabled: !appearance.operationBusy
                        }
                        Button {
                            text: "Choisir…"
                            enabled: !appearance.operationBusy
                            onClicked: wallpaperPicker.open()
                        }
                    }
                    Image {
                        Layout.fillWidth: true
                        Layout.maximumHeight: 180
                        fillMode: Image.PreserveAspectFit
                        source: wallpaperField.text === "" ? "" : "file://" + wallpaperField.text
                        visible: wallpaperField.text !== ""
                    }
                    Button {
                        text: "Appliquer"
                        enabled: !appearance.operationBusy && wallpaperField.text !== ""
                        onClicked: appearance.setWallpaper(wallpaperField.text)
                    }
                }
            }

            GroupBox {
                title: "Police"
                Layout.fillWidth: true

                ColumnLayout {
                    anchors.fill: parent
                    spacing: 8

                    StatField {
                        Layout.fillWidth: true
                        fieldLabel: "Famille et taille"
                        fieldValue: appearance.fontText
                    }
                    RowLayout {
                        Layout.fillWidth: true
                        spacing: 8

                        TextField {
                            id: fontFamilyField
                            Layout.fillWidth: true
                            placeholderText: "Famille (ex. Cantarell)"
                            enabled: !appearance.operationBusy
                        }
                        TextField {
                            id: fontSizeField
                            Layout.preferredWidth: 80
                            placeholderText: "Taille"
                            inputMethodHints: Qt.ImhDigitsOnly
                            enabled: !appearance.operationBusy
                        }
                    }
                    Button {
                        text: "Appliquer"
                        enabled: !appearance.operationBusy && fontFamilyField.text !== ""
                        onClicked: appearance.setFont(fontFamilyField.text, fontSizeField.text)
                    }
                }
            }
        }
    }
}
