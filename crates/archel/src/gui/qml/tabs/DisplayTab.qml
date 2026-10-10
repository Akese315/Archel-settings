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
    DisplayModel {
        id: display
    }

    Timer {
        interval: 1000
        running: true
        repeat: true
        onTriggered: {
            display.tick();
            display.countdownTick();
        }
    }
    Component.onCompleted: {
        display.tick();
        display.countdownTick();
    }

    CountdownModal {
        monitorName: display.countdownMonitor
        secondsLeft: display.countdownSeconds
        totalSeconds: 15
        visible: display.countdownVisible
        onKeepRequested: display.keepChange()
        onRevertRequested: display.revertChange()
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
                font.pixelSize: Math.round(Qt.font({}).pixelSize * 1.25)
            }

            PollBanner {
                bannerVisible: display.bannerVisible
                onDismissRequested: display.dismissBanner()
            }

            BusyIndicator {
                running: display.operationBusy
                visible: display.operationBusy
                Layout.alignment: Qt.AlignHCenter
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
                                text: "Mode"
                                Layout.preferredWidth: 120
                            }
                            ComboBox {
                                id: modeBox
                                Layout.fillWidth: true
                                model: modelData.available_modes
                                enabled: !display.operationBusy
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
                                enabled: !display.operationBusy
                            }
                        }
                        StatField {
                            Layout.fillWidth: true
                            fieldLabel: "Position"
                            fieldValue: modelData.position
                        }
                        Button {
                            text: "Appliquer"
                            enabled: !display.operationBusy
                            onClicked: {
                                var mode = modeBox.currentText !== undefined ? modeBox.currentText : "";
                                var at = mode.indexOf("@");
                                var resolution = at < 0 ? mode : mode.substring(0, at);
                                var refresh = at < 0 ? "" : mode.substring(at + 1).replace("Hz", "");
                                display.planChange(modelData.name, resolution, refresh, scaleField.text);
                            }
                        }
                    }
                }
            }
        }
    }
}
