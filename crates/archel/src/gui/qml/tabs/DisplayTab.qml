// Affichage tab: live Hyprland monitor inventory plus per-monitor
// mode/scale apply (writable, plan 06-08).
//
// The Timer runs unconditionally so the model keeps ticking while this
// tab is hidden (D-03/D-06); the countdown mirror rides the same tick.
// Every apply confirms first in the global ConfirmModal (D-12), then
// the CountdownModal below shows the D-16 keep-or-rollback window -
// the ONLY instantiation of CountdownModal in the shell (plan
// prohibition 3). While a write runs only this tab locks (D-15).
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import org.archel.settings

Item {
    // Id named `displayModel`, never `display`: AbstractButton's own
    // `display` enum property (value 2 = TextBesideIcon) shadows any
    // unqualified `display` lookup inside the Apply Button below.
    DisplayModel {
        id: displayModel
    }

    Timer {
        interval: 1000
        running: true
        repeat: true
        onTriggered: {
            displayModel.tick();
            displayModel.countdownTick();
        }
    }
    Component.onCompleted: {
        displayModel.tick();
        displayModel.countdownTick();
    }

    CountdownModal {
        monitorName: displayModel.countdownMonitor
        secondsLeft: displayModel.countdownSeconds
        totalSeconds: 15
        visible: displayModel.countdownVisible
        onKeepRequested: displayModel.keepChange()
        onRevertRequested: displayModel.revertChange()
        z: 100
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
                text: "Affichage"
                font.bold: true
                font.pixelSize: Math.round(Qt.application.font.pixelSize * 1.25)
            }

            PollBanner {
                bannerVisible: displayModel.bannerVisible
                onDismissRequested: displayModel.dismissBanner()
            }

            BusyIndicator {
                running: displayModel.operationBusy
                visible: displayModel.operationBusy
                Layout.alignment: Qt.AlignHCenter
            }

            Repeater {
                model: displayModel.monitorsJson === "" ? [] : JSON.parse(displayModel.monitorsJson)

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
                                text: "Mode"
                                Layout.preferredWidth: 120
                            }
                            ComboBox {
                                id: modeBox
                                Layout.fillWidth: true
                                model: modelData.available_modes
                                enabled: !displayModel.operationBusy
                            }
                        }

                        RowLayout {
                            Layout.fillWidth: true
                            spacing: 8

                            Label {
                                text: "Échelle"
                                Layout.preferredWidth: 120
                            }
                            TextField {
                                id: scaleField
                                Layout.fillWidth: true
                                placeholderText: modelData.scale === undefined ? "1.0" : String(modelData.scale)
                                enabled: !displayModel.operationBusy
                            }
                        }
                        StatField {
                            Layout.fillWidth: true
                            fieldLabel: "Position"
                            fieldValue: modelData.position
                        }
                        Button {
                            text: "Appliquer"
                            enabled: !displayModel.operationBusy
                            onClicked: {
                                var mode = modeBox.currentText !== undefined ? modeBox.currentText : "";
                                var at = mode.indexOf("@");
                                var resolution = at < 0 ? mode : mode.substring(0, at);
                                var refresh = at < 0 ? "" : mode.substring(at + 1).replace("Hz", "");
                                displayModel.planChange(modelData.name, resolution, refresh, scaleField.text);
                            }
                        }
                    }
                }
            }
        }
    }
}
