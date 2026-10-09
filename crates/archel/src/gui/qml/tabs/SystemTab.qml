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
                font.pixelSize: Math.round(Qt.font({}).pixelSize * 1.25)
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
                    Button {
                        id: coresToggle
                        text: "Cœurs"
                        checkable: true
                        visible: system.coreCount !== 1
                    }
                    ColumnLayout {
                        visible: coresToggle.checked && system.coreCount > 1
                        spacing: 8

                        Repeater {
                            model: system.coresJson === "" ? [] : JSON.parse(system.coresJson)
                            delegate: StatField {
                                Layout.fillWidth: true
                                fieldLabel: modelData.name
                                fieldValue: modelData.usage.toFixed(1) + " % · " + modelData.freq + " MHz"
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
