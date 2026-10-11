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
    id: root

    // Singleton-free wiring (spec): instantiated once in Main.qml,
    // passed to this tab only — the other tabs read the palette.
    property ThemeModel theme

    // The six catppuccin accents (accent-color.html mockup). Keys match
    // models/theme.rs ACCENTS; hex = the dark-mode full color used for
    // the swatch fill (display only, the real palette comes from Rust).
    ListModel {
        id: accentModel
        ListElement { key: "blue";  name: "Bleu";     hex: "#89b4fa" }
        ListElement { key: "mauve"; name: "Mauve";    hex: "#cba6f7" }
        ListElement { key: "green"; name: "Vert";     hex: "#a6e3a1" }
        ListElement { key: "teal";  name: "Sarcelle"; hex: "#94e2d5" }
        ListElement { key: "pink";  name: "Rose";     hex: "#f5c2e7" }
        ListElement { key: "peach"; name: "Pêche";    hex: "#fab387" }
    }

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
        onAccepted: {
            var local = selectedFile.toLocalFile()
            if (local !== "")
                wallpaperField.text = local
        }
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

            PageHeader {
                Layout.fillWidth: true
                pageTitle: "Apparence"
                pageSubtitle: "Effets, fond d'écran et police"
            }

            SectionCard {
                title: "Thème"
                Layout.fillWidth: true

                ColumnLayout {
                    anchors.fill: parent
                    spacing: 8

                    RowLayout {
                        Layout.fillWidth: true
                        spacing: 8

                        Label {
                            text: "Mode sombre"
                            Layout.fillWidth: true
                        }
                        Switch {
                            checked: root.theme.darkMode
                            onToggled: root.theme.setDark(checked)
                        }
                    }

                    Label {
                        text: "Accent"
                        color: root.palette.placeholderText
                        Layout.fillWidth: true
                    }
                    RowLayout {
                        Layout.fillWidth: true
                        spacing: 10

                        Repeater {
                            model: accentModel

                            AbstractButton {
                                id: swatch
                                required property string key
                                required property string hex
                                required property string name
                                implicitWidth: 28
                                implicitHeight: 28
                                Accessible.role: Accessible.Button
                                Accessible.name: swatch.name

                                background: Rectangle {
                                    radius: swatch.width / 2
                                    color: swatch.hex
                                    border.width: root.theme.accentKey === swatch.key ? 3 : 1
                                    border.color: root.theme.accentKey === swatch.key
                                        ? root.theme.colorBrightText
                                        : root.palette.mid
                                    Rectangle {
                                        anchors.fill: parent
                                        anchors.margins: -3
                                        radius: swatch.width / 2 + 3
                                        border.width: 2
                                        border.color: root.palette.accent
                                        visible: swatch.activeFocus
                                    }
                                }
                                ToolTip.text: swatch.name
                                ToolTip.visible: hovered
                                onClicked: root.theme.setAccent(swatch.key)
                            }
                        }

                        Item { Layout.fillWidth: true }
                        Label {
                            text: {
                                var names = { blue: "Bleu", mauve: "Mauve", green: "Vert",
                                              teal: "Sarcelle", pink: "Rose", peach: "Pêche" };
                                return names[root.theme.accentKey] || root.theme.accentKey;
                            }
                            color: root.palette.placeholderText
                        }
                    }
                }
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

            SectionCard {
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

            SectionCard {
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

            SectionCard {
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
