// Son tab: default sink/source pickers over the live audio_list
// devices, with the plan-07 switches behind the D-12 confirm modal.
//
// The Timer runs unconditionally so the model keeps ticking while this
// tab is hidden (D-03/D-06). Failures read the empty lists plus the
// once-per-episode banner (D-04/D-05). Switching resolves the picked
// name against a fresh list first: an unknown name renders the core
// error string inline under the picker, never in a modal. While a
// switch is in flight both pickers disable themselves and a spinner
// shows (D-15); the refresh after the switch confirms the new default.
// Device names elide in the middle with the full text in a ToolTip
// (UI-SPEC long-text backstop).
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import org.archel.settings

Item {
    AudioModel {
        id: audio
    }

    Timer {
        interval: 1000
        running: true
        repeat: true
        onTriggered: audio.tick()
    }
    Component.onCompleted: audio.tick()

    // Index of the default device in a parsed device list, or -1 when
    // the list is empty / has no default (honest, never guessed).
    function defaultIndex(rows) {
        for (var i = 0; i < rows.length; i++) {
            if (rows[i].default) {
                return i
            }
        }
        return -1
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
                text: "Son"
                font.bold: true
                font.pixelSize: Math.round(Qt.font({}).pixelSize * 1.25)
            }

            PollBanner {
                bannerVisible: audio.bannerVisible
                onDismissRequested: audio.dismissBanner()
            }

            GroupBox {
                title: "Lecture"
                Layout.fillWidth: true

                ColumnLayout {
                    anchors.fill: parent
                    spacing: 8

                    RowLayout {
                        spacing: 8
                        Layout.fillWidth: true

                        ComboBox {
                            id: sinkPicker
                            Layout.fillWidth: true
                            enabled: !audio.operationBusy
                            model: audio.sinksJson === "" ? [] : JSON.parse(audio.sinksJson)
                            textRole: "description"
                            currentIndex: defaultIndex(audio.sinksJson === "" ? [] : JSON.parse(audio.sinksJson))
                            ToolTip.visible: hovered
                            ToolTip.text: currentIndex >= 0 ? model[currentIndex].name : ""
                            onActivated: function (index) {
                                var rows = audio.sinksJson === "" ? [] : JSON.parse(audio.sinksJson)
                                if (index >= 0 && index < rows.length) {
                                    audio.setDefaultOutput(rows[index].name)
                                }
                            }
                        }
                        BusyIndicator {
                            visible: audio.operationBusy
                            running: audio.operationBusy
                        }
                    }
                    Label {
                        Layout.fillWidth: true
                        visible: audio.outputErrorText !== ""
                        text: audio.outputErrorText
                        wrapMode: Label.WrapAnywhere
                    }
                }
            }

            GroupBox {
                title: "Enregistrement"
                Layout.fillWidth: true

                ColumnLayout {
                    anchors.fill: parent
                    spacing: 8

                    RowLayout {
                        spacing: 8
                        Layout.fillWidth: true

                        ComboBox {
                            id: sourcePicker
                            Layout.fillWidth: true
                            enabled: !audio.operationBusy
                            model: audio.sourcesJson === "" ? [] : JSON.parse(audio.sourcesJson)
                            textRole: "description"
                            currentIndex: defaultIndex(audio.sourcesJson === "" ? [] : JSON.parse(audio.sourcesJson))
                            ToolTip.visible: hovered
                            ToolTip.text: currentIndex >= 0 ? model[currentIndex].name : ""
                            onActivated: function (index) {
                                var rows = audio.sourcesJson === "" ? [] : JSON.parse(audio.sourcesJson)
                                if (index >= 0 && index < rows.length) {
                                    audio.setDefaultInput(rows[index].name)
                                }
                            }
                        }
                        BusyIndicator {
                            visible: audio.operationBusy
                            running: audio.operationBusy
                        }
                    }
                    Label {
                        Layout.fillWidth: true
                        visible: audio.inputErrorText !== ""
                        text: audio.inputErrorText
                        wrapMode: Label.WrapAnywhere
                    }
                }
            }
        }
    }
}
