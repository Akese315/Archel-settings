// Affichage tab: live Hyprland monitor inventory (read-only).
//
// The Timer runs unconditionally so the model keeps ticking while this
// tab is hidden (D-03/D-06). Pre-first-tick values read the ellipsis
// token; failures read N/A plus the once-per-episode banner (D-04/D-05).
// The resolution/refresh ComboBoxes are populated from the core-exposed
// option lists but stay disabled until plan 08 wires writes.
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import org.archel.settings

Item {
    DisplayModel {
        id: display
    }

    Timer {
        interval: 1000
        running: true
        repeat: true
        onTriggered: display.tick()
    }
    Component.onCompleted: display.tick()

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
                text: "Affichage"
                font.bold: true
                font.pixelSize: Math.round(Qt.font({}).pixelSize * 1.25)
            }

            PollBanner {
                bannerVisible: display.bannerVisible
                onDismissRequested: display.dismissBanner()
            }

            Repeater {
                model: display.monitorsJson === "" ? [] : JSON.parse(display.monitorsJson)

                GroupBox {
                    Layout.fillWidth: true
                    title: modelData.name

                    ColumnLayout {
                        anchors.fill: parent
                        spacing: 8

                        StatField {
                            Layout.fillWidth: true
                            fieldLabel: "Mode actuel"
                            fieldValue: modelData.summary
                        }

                        RowLayout {
                            Layout.fillWidth: true
                            spacing: 8

                            Label {
                                text: "Résolution"
                                Layout.preferredWidth: 120
                            }
                            ComboBox {
                                Layout.fillWidth: true
                                model: modelData.available_modes
                                enabled: false
                            }
                        }

                        RowLayout {
                            Layout.fillWidth: true
                            spacing: 8

                            Label {
                                text: "Fréquence"
                                Layout.preferredWidth: 120
                            }
                            ComboBox {
                                Layout.fillWidth: true
                                model: modelData.available_modes
                                enabled: false
                            }
                        }

                        StatField {
                            Layout.fillWidth: true
                            fieldLabel: "Échelle"
                            fieldValue: modelData.scale === undefined ? "" : String(modelData.scale)
                        }
                        StatField {
                            Layout.fillWidth: true
                            fieldLabel: "Position"
                            fieldValue: modelData.position
                        }
                    }
                }
            }
        }
    }
}
