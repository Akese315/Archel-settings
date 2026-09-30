// Archel settings spike (D-14 bar): vertical tabs + live label.
//
// Pattern: QtQuick.Controls TabBar is horizontal-only, so vertical tabs are
// a Column of TabButtons driving a StackLayout. The live label binds to the
// Rust SpikeModel.tick_label Q_PROPERTY; a 1 Hz Timer calls spike.tick()
// and NOTIFY re-renders the text. Tab labels are French (D-06, one language
// everywhere); this QML is fully static — no untrusted strings.
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import org.archel.spike

ApplicationWindow {
    visible: true
    width: 900
    height: 620
    title: "Archel — Réglages (spike)"

    SpikeModel {
        id: spike
    }

    RowLayout {
        anchors.fill: parent
        anchors.margins: 12
        spacing: 12

        // Vertical tabs (left rail).
        ColumnLayout {
            Layout.preferredWidth: 180
            Layout.fillHeight: true
            spacing: 4

            TabButton {
                text: "Réseau"
                checkable: true
                autoExclusive: true
                checked: true
                onClicked: stack.currentIndex = 0
            }
            TabButton {
                text: "Alimentation"
                checkable: true
                autoExclusive: true
                onClicked: stack.currentIndex = 1
            }
            TabButton {
                text: "Bluetooth"
                checkable: true
                autoExclusive: true
                onClicked: stack.currentIndex = 2
            }
            TabButton {
                text: "Apparence"
                checkable: true
                autoExclusive: true
                onClicked: stack.currentIndex = 3
            }
            TabButton {
                text: "Audio"
                checkable: true
                autoExclusive: true
                onClicked: stack.currentIndex = 4
            }
            Item {
                Layout.fillHeight: true
            }
        }

        // Tab content.
        StackLayout {
            id: stack
            currentIndex: 0
            Layout.fillWidth: true
            Layout.fillHeight: true

            ColumnLayout {
                Label {
                    text: "Réseau (spike)"
                    font.bold: true
                }
                Label {
                    text: spike.tick_label
                }
            }
            Label { text: "Alimentation (spike)" }
            Label { text: "Bluetooth (spike)" }
            Label { text: "Apparence (spike)" }
            Label { text: "Audio (spike)" }
        }
    }

    Timer {
        interval: 1000
        running: true
        repeat: true
        onTriggered: spike.tick()
    }
}
