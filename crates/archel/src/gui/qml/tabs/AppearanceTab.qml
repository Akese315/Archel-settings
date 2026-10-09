// Apparence tab: blur switch, opacity leaves, wallpaper path and font
// stores (read-only; plan 08 owns writes).
//
// The Timer runs unconditionally so the model keeps ticking while this
// tab is hidden (D-03/D-06). Pre-first-tick values read the ellipsis
// token; failures read N/A plus the once-per-episode banner (D-04/D-05).
// Controls reflect props each tick but perform no writes: no invokable
// slots exist yet, so every control stays disabled.
import QtQuick
import QtQuick.Controls
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
                            checked: appearance.blurText === "on"
                            enabled: false
                        }
                    }
                    StatField {
                        Layout.fillWidth: true
                        fieldLabel: "État"
                        fieldValue: appearance.blurText
                    }

                    RowLayout {
                        Layout.fillWidth: true
                        spacing: 8

                        Label {
                            text: "Opacité"
                            Layout.fillWidth: true
                        }
                        Slider {
                            from: 0
                            to: 1
                            stepSize: 0.01
                            value: {
                                if (appearance.opacityText === "")
                                    return 1;
                                var active = parseFloat(appearance.opacityText.split("/")[0]);
                                return isNaN(active) ? 1 : active;
                            }
                            enabled: false
                        }
                    }
                    StatField {
                        Layout.fillWidth: true
                        fieldLabel: "Active / inactive"
                        fieldValue: appearance.opacityText
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
                }
            }
        }
    }
}
