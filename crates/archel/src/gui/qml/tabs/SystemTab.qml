// Système tab: live 1 Hz CPU/RAM sparklines over a 60 s rolling window,
// aggregate line plus per-core expander, representative temps only
// (D-09/D-10/D-11).
//
// The Timer runs unconditionally — never bound to the visible tab — so
// the model keeps ticking while this tab is hidden, the window is
// minimized, or the app sits on another workspace; StackLayout gates
// rendering only (D-03/D-06). Pre-first-tick values read the ellipsis
// token; failures read N/A plus the once-per-episode banner (D-04/D-05).
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import org.archel.settings

Item {
    SystemModel {
        id: system
    }

    Timer {
        interval: 1000
        running: true
        repeat: true
        onTriggered: system.tick()
    }
    Component.onCompleted: system.tick()

    SystemPalette {
        id: sysPalette
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
                text: "Système"
                font.bold: true
                font.pixelSize: Math.round(Qt.application.font.pixelSize * 1.25)
            }

            PollBanner {
                bannerVisible: system.bannerVisible
                onDismissRequested: system.dismissBanner()
            }

            GroupBox {
                title: "Processeur"
                Layout.fillWidth: true

                ColumnLayout {
                    anchors.fill: parent
                    spacing: 8

                    Sparkline {
                        Layout.fillWidth: true
                        Layout.preferredHeight: 60
                        lineColor: sysPalette.highlight
                        samples: system.cpuSeries === "" ? [] : JSON.parse(system.cpuSeries)
                    }
                    StatField {
                        Layout.fillWidth: true
                        fieldLabel: "Utilisation"
                        fieldValue: system.cpuText
                    }
                    StatField {
                        Layout.fillWidth: true
                        fieldLabel: "Horloge"
                        fieldValue: system.clockText
                    }
                    StatField {
                        Layout.fillWidth: true
                        fieldLabel: "Modèle"
                        fieldValue: system.cpuModelText
                    }
                    StatField {
                        Layout.fillWidth: true
                        fieldLabel: "Microcode"
                        fieldValue: system.cpuDriverText
                    }
                    Button {
                        id: coresToggle
                        text: "Cœurs"
                        checkable: true
                        visible: system.coreCount !== 1
                    }
                    // Per-core matrix: one compact tile per core with
                    // its live percentage, frequency and a load bar.
                    // Flow wraps the tiles as the window allows; with
                    // 16 logical cores the matrix stays readable where
                    // a one-row-per-core list needed a full screen.
                    Flow {
                        Layout.fillWidth: true
                        spacing: 6
                        visible: coresToggle.checked && system.coreCount > 1

                        Repeater {
                            model: system.coresJson === "" ? [] : JSON.parse(system.coresJson)
                            delegate: Rectangle {
                                width: 92
                                height: 72
                                radius: 6
                                color: sysPalette.alternateBase
                                border.color: sysPalette.mid
                                border.width: 1

                                Column {
                                    anchors.centerIn: parent
                                    spacing: 2

                                    Label {
                                        anchors.horizontalCenter: parent.horizontalCenter
                                        text: modelData.name
                                        font.pixelSize: Math.round(Qt.application.font.pixelSize * 0.85)
                                        opacity: 0.7
                                    }
                                    Label {
                                        anchors.horizontalCenter: parent.horizontalCenter
                                        text: modelData.usage.toFixed(0) + " %"
                                        font.bold: true
                                        font.pixelSize: Math.round(Qt.application.font.pixelSize * 1.25)
                                        color: modelData.usage > 85 ? "#e01b24"
                                              : modelData.usage > 50 ? "#ff7800"
                                              : sysPalette.text
                                    }
                                    Rectangle {
                                        anchors.horizontalCenter: parent.horizontalCenter
                                        width: 64
                                        height: 4
                                        radius: 2
                                        color: sysPalette.mid
                                        Rectangle {
                                            anchors.left: parent.left
                                            anchors.verticalCenter: parent.verticalCenter
                                            width: Math.min(64, 64 * modelData.usage / 100)
                                            height: 4
                                            radius: 2
                                            color: modelData.usage > 85 ? "#e01b24"
                                                  : modelData.usage > 50 ? "#ff7800"
                                                  : "#2ec27e"
                                        }
                                    }
                                    Label {
                                        anchors.horizontalCenter: parent.horizontalCenter
                                        text: modelData.freq + " MHz"
                                        font.pixelSize: Math.round(Qt.application.font.pixelSize * 0.7)
                                        opacity: 0.6
                                    }
                                }
                            }
                        }
                    }
                }
            }

            GroupBox {
                title: "Mémoire"
                Layout.fillWidth: true

                ColumnLayout {
                    anchors.fill: parent
                    spacing: 8

                    Sparkline {
                        Layout.fillWidth: true
                        Layout.preferredHeight: 60
                        lineColor: "#2ec27e"
                        samples: system.ramSeries === "" ? [] : JSON.parse(system.ramSeries)
                    }
                    StatField {
                        Layout.fillWidth: true
                        fieldLabel: "Utilisée / Totale"
                        fieldValue: system.ramText
                    }
                }
            }

            GroupBox {
                title: "Disque système"
                Layout.fillWidth: true

                ColumnLayout {
                    anchors.fill: parent
                    spacing: 8

                    StatField {
                        Layout.fillWidth: true
                        fieldLabel: "Utilisé / Total"
                        fieldValue: system.diskText
                    }
                }
            }

            GroupBox {
                title: "Températures"
                Layout.fillWidth: true

                ColumnLayout {
                    anchors.fill: parent
                    spacing: 8

                    StatField {
                        Layout.fillWidth: true
                        fieldLabel: "CPU"
                        fieldValue: system.tempCpuText
                    }
                    StatField {
                        Layout.fillWidth: true
                        fieldLabel: "GPU"
                        fieldValue: system.tempGpuText
                    }
                }
            }

            GroupBox {
                title: "Carte graphique"
                Layout.fillWidth: true

                ColumnLayout {
                    anchors.fill: parent
                    spacing: 8

                    StatField {
                        Layout.fillWidth: true
                        fieldLabel: "Modèle / VRAM"
                        fieldValue: system.gpuText
                    }
                    StatField {
                        Layout.fillWidth: true
                        fieldLabel: "Pilote"
                        fieldValue: system.gpuDriverText
                    }
                    StatField {
                        Layout.fillWidth: true
                        fieldLabel: "CUDA"
                        fieldValue: system.cudaText
                    }
                }
            }

            GroupBox {
                title: "Noyau"
                Layout.fillWidth: true

                ColumnLayout {
                    anchors.fill: parent
                    spacing: 8

                    StatField {
                        Layout.fillWidth: true
                        fieldLabel: "Version"
                        fieldValue: system.kernelText
                    }
                }
            }
        }
    }
}
