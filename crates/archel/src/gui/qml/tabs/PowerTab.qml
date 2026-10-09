// Alimentation tab: active profile plus backend tag (read-only).
//
// The Timer runs unconditionally so the model keeps ticking while this
// tab is hidden (D-03/D-06). Pre-first-tick values read the ellipsis
// token; failures read N/A plus the once-per-episode banner (D-04/D-05).
// The profile list renders read-only at this stage: writes land in
// plan 07 behind the D-12 confirm modal.
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import org.archel.settings

Item {
    PowerModel {
        id: power
    }

    Timer {
        interval: 1000
        running: true
        repeat: true
        onTriggered: power.tick()
    }
    Component.onCompleted: power.tick()

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
                text: "Alimentation"
                font.bold: true
                font.pixelSize: Math.round(Qt.font({}).pixelSize * 1.25)
            }

            PollBanner {
                bannerVisible: power.bannerVisible
                onDismissRequested: power.dismissBanner()
            }

            GroupBox {
                title: "Profil"
                Layout.fillWidth: true

                ColumnLayout {
                    anchors.fill: parent
                    spacing: 8

                    Repeater {
                        model: power.profilesJson === "" ? [] : JSON.parse(power.profilesJson)
                        delegate: RadioButton {
                            text: modelData.id
                            checked: modelData.active
                            enabled: false
                        }
                    }
                    StatField {
                        Layout.fillWidth: true
                        fieldLabel: "Profil actif"
                        fieldValue: power.profileText
                    }
                }
            }

            GroupBox {
                title: "Pilote"
                Layout.fillWidth: true

                ColumnLayout {
                    anchors.fill: parent
                    spacing: 8

                    StatField {
                        Layout.fillWidth: true
                        fieldLabel: "Backend"
                        fieldValue: power.backendText
                    }
                    StatField {
                        Layout.fillWidth: true
                        visible: power.noteText !== ""
                        fieldLabel: "Note"
                        fieldValue: power.noteText
                    }
                }
            }
        }
    }
}
