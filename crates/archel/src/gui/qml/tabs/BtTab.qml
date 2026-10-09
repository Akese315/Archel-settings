// Bluetooth tab: known-device list plus the 12 s burst scan (D-08).
//
// The 1 Hz Timer runs unconditionally but the model pauses ticks while
// a burst scan runs, so the burst gets the pool slot alone; the
// Rechercher button stays disabled with a BusyIndicator during the
// burst and the list replaces only at burst end (no progressive fill,
// RESEARCH Core Gap #2 decision). Failures read the empty list plus
// the once-per-episode banner (D-04/D-05). Device names elide in the
// middle with the full text in a ToolTip (UI-SPEC long-text backstop).
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import org.archel.settings

Item {
    BtModel {
        id: bt
    }

    Timer {
        interval: 1000
        running: true
        repeat: true
        onTriggered: bt.tick()
    }
    Component.onCompleted: bt.tick()

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
                text: "Bluetooth"
                font.bold: true
                font.pixelSize: Math.round(Qt.font({}).pixelSize * 1.25)
            }

            PollBanner {
                bannerVisible: bt.bannerVisible
                onDismissRequested: bt.dismissBanner()
            }

            GroupBox {
                title: "Appareils"
                Layout.fillWidth: true

                ColumnLayout {
                    anchors.fill: parent
                    spacing: 8

                    ListView {
                        id: deviceList
                        Layout.fillWidth: true
                        Layout.preferredHeight: contentHeight
                        interactive: false
                        clip: true
                        model: bt.devicesJson === "" ? [] : JSON.parse(bt.devicesJson)
                        delegate: ItemDelegate {
                            width: deviceList.width
                            hoverEnabled: true
                            ToolTip.visible: hovered
                            ToolTip.text: modelData.name + " · " + modelData.address
                            contentItem: RowLayout {
                                spacing: 8
                                Label {
                                    text: modelData.name
                                    Layout.fillWidth: true
                                    elide: Text.ElideMiddle
                                }
                                Label {
                                    text: modelData.state
                                    font.pixelSize: Math.round(Qt.font({}).pixelSize * 0.85)
                                    opacity: 0.6
                                }
                                Label {
                                    text: modelData.signal
                                    font.pixelSize: Math.round(Qt.font({}).pixelSize * 0.85)
                                    opacity: 0.6
                                }
                            }
                        }
                    }

                    ColumnLayout {
                        Layout.fillWidth: true
                        spacing: 8
                        visible: !bt.scanning && bt.devicesJson !== "" && JSON.parse(bt.devicesJson).length === 0

                        Label {
                            text: "Aucun appareil détecté"
                            font.bold: true
                            Layout.fillWidth: true
                            horizontalAlignment: Text.AlignHCenter
                        }
                        Label {
                            text: "Appuyez sur « Rechercher » pour lancer une analyse de 12 secondes."
                            wrapMode: Text.Wrap
                            Layout.fillWidth: true
                            horizontalAlignment: Text.AlignHCenter
                        }
                    }

                    RowLayout {
                        spacing: 8

                        Button {
                            text: "Rechercher"
                            enabled: !bt.scanning
                            onClicked: bt.scanNow()
                        }
                        BusyIndicator {
                            visible: bt.scanning
                            running: bt.scanning
                        }
                    }
                }
            }
        }
    }
}
